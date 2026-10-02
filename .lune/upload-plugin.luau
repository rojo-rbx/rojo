local fs = require("@lune/fs")
local process = require("@lune/process")
local stdio = require("@lune/stdio")

local luau_execute = require("./opencloud-execute")

local UNIVERSE_ID = process.env["RBX_UNIVERSE_ID"]
local PLACE_ID = process.env["RBX_PLACE_ID"]

local version_string = fs.readFile("plugin/Version.txt")
local versions = { string.match(version_string, "^v?(%d+)%.(%d+)%.(%d+)(.*)$") }
if versions[4] ~= "" then
	print("This release is a pre-release. Skipping uploading plugin.")
	process.exit(0)
end

local plugin_path = process.args[1]
assert(
	typeof(plugin_path) == "string",
	"no plugin path provided, expected usage is `lune run upload-plugin [PATH TO RBXM]`."
)

-- For local testing
if process.env["CI"] ~= "true" then
	local rojo = process.exec("rojo", { "build", "plugin.project.json", "--output", plugin_path })
	if not rojo.ok then
		stdio.ewrite("plugin upload failed because: could not build plugin.rbxm\n\n")
		stdio.ewrite(rojo.stderr)
		stdio.ewrite("\n")
		process.exit(1)
	end
else
	assert(fs.isFile(plugin_path), `Plugin file did not exist at {plugin_path}`)
end
local plugin_content = fs.readFile(plugin_path)

local engine_script = fs.readFile(".lune/scripts/plugin-upload.luau")

print("Creating task to upload plugin")
local task = luau_execute.create_task_latest(UNIVERSE_ID, PLACE_ID, engine_script, 300, false, plugin_content)

print("Waiting for task to finish")
local success = luau_execute.await_finish(task)
if not success then
	local error = luau_execute.get_error(task)
	assert(error, "could not fetch error from task")
	stdio.ewrite("plugin upload failed because: task did not finish successfully\n\n")
	stdio.ewrite(error.code)
	stdio.ewrite("\n")
	stdio.ewrite(error.message)
	stdio.ewrite("\n")
	process.exit(1)
end

print("Output from task:\n")
for _, log in luau_execute.get_structured_logs(task) do
	if log.messageType == "ERROR" then
		stdio.write(stdio.color("red"))
		stdio.write(log.message)
		stdio.write("\n")
		stdio.write(stdio.color("reset"))
	elseif log.messageType == "INFO" then
		stdio.write(stdio.color("cyan"))
		stdio.write(log.message)
		stdio.write("\n")
		stdio.write(stdio.color("reset"))
	elseif log.messageType == "WARNING" then
		stdio.write(stdio.color("yellow"))
		stdio.write(log.message)
		stdio.write("\n")
		stdio.write(stdio.color("reset"))
	else
		stdio.write(stdio.color("reset"))
		stdio.write(log.message)
		stdio.write("\n")
		stdio.write(stdio.color("reset"))
	end
end
