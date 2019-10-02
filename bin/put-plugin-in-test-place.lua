local pluginPath, placePath = ...

local plugin = remodel.readModelFile(pluginPath)[1]
local place = remodel.readPlaceFile(placePath)

plugin.Parent = place:GetService("ReplicatedStorage")

remodel.writePlaceFile(place, placePath)