local serde = require("@lune/serde")
local net = require("@lune/net")
local stdio = require("@lune/stdio")
local process = require("@lune/process")
local fs = require("@lune/fs")

local luau_execute = require("./opencloud-execute")

local TEST_SCRIPT = fs.readFile("plugin/run-tests.server.lua")

local PATH_VERSION_MATCH = "assets/%d+/versions/(.+)"

local UNIVERSE_ID = process.env["RBX_UNIVERSE_ID"]
local PLACE_ID = process.env["RBX_PLACE_ID"]
local API_KEY = process.env["RBX_API_KEY"]

if not UNIVERSE_ID then
	error("no universe ID specified. try providing one with the env var `RBX_UNIVERSE_ID`")
end
if not PLACE_ID then
	error("no place ID specified. try providing one with the env var `RBX_PLACE_ID`")
end
if not API_KEY then
	error("no API key specified. try providing one with the env var `RBX_API_KEY`")
end

--stylua: ignore
local upload_result = process.exec("cargo", {
	"run", "--",
	"upload", "plugin/test-place.project.json",
	"--api_key", API_KEY,
	"--universe_id", UNIVERSE_ID,
	"--asset_id", PLACE_ID
}, {
	stdio = "none"
})

if not upload_result.ok then
	print("Failed to upload plugin test place")
	print("Not dumping stdout or stderr to avoid leaking secrets")
	process.exit(1)
end

-- This is /probably/ not necessary because Rojo generally does not have enough
-- activity that there will be multiple CI runs happening at once, but
-- it's better safe than sorry.
local version_response = net.request({
	method = "GET",
	url = `https://apis.roblox.com/assets/v1/assets/{PLACE_ID}/versions`,
	query = {
		maxPageSize = 1,
	},
	headers = {
		["User-Agent"] = `Rojo/PluginTesting 1.0.0; {_VERSION}`,
		["x-api-key"] = API_KEY,
	},
})

if not version_response.ok then
	error(
		`Failed to fetch version of Roblox place to run tests on because: {version_response.statusCode} - {version_response.statusMessage}\n{version_response.body}`
	)
end

local place_version_raw = serde.decode("json", version_response.body).assetVersions[1].path
assert(typeof(place_version_raw) == "string", "the result from asset version endpoint was not as expected")

local place_version = string.match(place_version_raw, PATH_VERSION_MATCH)

local task = luau_execute.create_task_versioned(UNIVERSE_ID, PLACE_ID, place_version, TEST_SCRIPT)
print(`Running test script on {UNIVERSE_ID}/{PLACE_ID}@{place_version}`)
print(`Task ID: {luau_execute.task_id(task)}`)

luau_execute.await_finish(task)
print("Output from task:\n")
local logs = luau_execute.get_structured_logs(task)
for _, log in logs do
	if log.messageType == "OUTPUT" or log.messageType == "MESSAGE_TYPE_UNSPECIFIED" then
		stdio.write(stdio.color("reset"))
	elseif log.messageType == "INFO" then
		stdio.write(stdio.color("cyan"))
	elseif log.messageType == "WARNING" then
		stdio.write(stdio.color("yellow"))
	elseif log.messageType == "ERROR" then
		stdio.write(stdio.color("red"))
	end
	stdio.write(log.message)
	stdio.write(`{stdio.color("reset")}\n`)
end

local results = luau_execute.get_output(task)[1]
if not results then
	error("plugin tests did not return any results")
end

local status = luau_execute.check_status(task)
if status == "COMPLETE" then
	if results.failureCount == 0 then
		process.exit(0)
	else
		process.exit(1)
	end
else
	print()
	print("Task did not finish successfully")
	local err = luau_execute.get_error(task)
	if err then
		print(`Error from task: {err.code}`)
		print(err.message)
	end
	process.exit(1)
end
