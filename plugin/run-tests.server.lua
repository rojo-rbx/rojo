local ReplicatedStorage = game:GetService("ReplicatedStorage")

local TestEZ = require(ReplicatedStorage:WaitForChild("Packages", 10):WaitForChild("TestEZ", 10))

local Rojo = ReplicatedStorage:WaitForChild("Rojo", 10)

local Settings = require(Rojo.Plugin.Settings)
Settings:set("logLevel", "Trace")
Settings:set("typecheckingEnabled", true)

require(Rojo.Plugin.runTests)(TestEZ)
