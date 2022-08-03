local ReplicatedStorage = game:GetService("ReplicatedStorage")

local TestEZ = require(ReplicatedStorage.Packages.TestEZ)

local Rojo = ReplicatedStorage.Rojo

local DevSettings = require(Rojo.Plugin.DevSettings)

local setDevSettings = not DevSettings:hasChangedValues()

if setDevSettings then
	DevSettings:createTestSettings()
end

require(Rojo.Plugin.runTests)(TestEZ)

if setDevSettings then
	DevSettings:resetValues()
end
