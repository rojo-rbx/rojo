--[[
	Loads our library and all of its dependencies, then runs tests using TestEZ.
]]

-- If you add any dependencies, add them to this table so they'll be loaded!
local LOAD_MODULES = {
	{"src", "plugin"},
	{"modules/promise/lib", "Promise"},
	{"modules/testez/lib", "TestEZ"},
}

-- This makes sure we can load Lemur and other libraries that depend on init.lua
package.path = package.path .. ";?/init.lua"

-- If this fails, make sure you've run `lua bin/install-dependencies.lua` first!
local lemur = require("modules.lemur")

-- Create a virtual Roblox tree
local habitat = lemur.Habitat.new()

-- We'll put all of our library code and dependencies here
local Root = lemur.Instance.new("Folder")
Root.Name = "Root"

-- Load all of the modules specified above
for _, module in ipairs(LOAD_MODULES) do
	local container = habitat:loadFromFs(module[1])
	container.Name = module[2]
	container.Parent = Root
end

-- Load TestEZ and run our tests
local TestEZ = habitat:require(Root.TestEZ)

local results = TestEZ.TestBootstrap:run(Root.plugin, TestEZ.Reporters.TextReporter)

-- Did something go wrong?
if results.failureCount > 0 then
	os.exit(1)
end