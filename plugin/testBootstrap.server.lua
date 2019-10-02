local ReplicatedStorage = game:GetService("ReplicatedStorage")

local TestEZ = require(ReplicatedStorage.TestEZ)

local Rojo = ReplicatedStorage.Rojo

local DevSettings = require(Rojo.Plugin.DevSettings)

local setDevSettings = not DevSettings:hasChangedValues()

if setDevSettings then
	DevSettings:createTestSettings()
end

TestEZ.TestBootstrap:run({ Rojo.Plugin, Rojo.Http, Rojo.Log })

if setDevSettings then
	DevSettings:resetValues()
end