local pluginPath, testezPath = ...

local plugin = remodel.readModelFile(pluginPath)[1]
local testez = remodel.readModelFile(testezPath)[1]

local marker = Instance.new("Folder")
marker.Name = "ROJO_DEV_BUILD"
marker.Parent = plugin

testez.Parent = plugin

remodel.writeModelFile(plugin, pluginPath)