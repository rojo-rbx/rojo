local loadEnvironment = require("loadEnvironment")

local testPath = assert((...), "Please specify a path to a test file.")

local habitat = loadEnvironment()

local testModule = habitat:loadFromFs(testPath)

if testModule == nil then
	error("Couldn't find test file at " .. testPath)
end

print("Starting test module.")

habitat:require(testModule)