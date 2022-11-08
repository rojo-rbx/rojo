local ReplicatedStorage = game:GetService("ReplicatedStorage")

local TestEZ = require(ReplicatedStorage.Packages.TestEZ)

local Rojo = ReplicatedStorage.Rojo

local Settings = require(Rojo.Plugin.Settings)
Settings:set("logLevel", "Trace")
Settings:set("typecheckingEnabled", true)

require(Rojo.Plugin.runTests)(TestEZ)
