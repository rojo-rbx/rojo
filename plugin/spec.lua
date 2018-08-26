--[[
	Loads our library and all of its dependencies, then runs tests using TestEZ.
]]

local loadEnvironment = require("loadEnvironment")

local habitat, modules = loadEnvironment()

-- Load TestEZ and run our tests
local TestEZ = habitat:require(modules.TestEZ)

local results = TestEZ.TestBootstrap:run(modules.Rojo, TestEZ.Reporters.TextReporter)

-- Did something go wrong?
if results.failureCount > 0 then
	os.exit(1)
end