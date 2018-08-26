--[[
	Loads the Rojo plugin and all of its dependencies.
]]

local function loadEnvironment()
	-- If you add any dependencies, add them to this table so they'll be loaded!
	local LOAD_MODULES = {
		{"src", "Rojo"},
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
	local modules = lemur.Instance.new("Folder")
	modules.Name = "Modules"
	modules.Parent = habitat.game:GetService("ReplicatedStorage")

	-- Load all of the modules specified above
	for _, module in ipairs(LOAD_MODULES) do
		local container = habitat:loadFromFs(module[1])
		container.Name = module[2]
		container.Parent = modules
	end

	return habitat, modules
end

return loadEnvironment