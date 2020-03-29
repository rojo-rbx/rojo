local ReplicatedStorage = game:GetService("ReplicatedStorage")

local LIB_ROOT = ReplicatedStorage.RbxDom

local TestEZ = require(ReplicatedStorage.TestEZ)

TestEZ.TestBootstrap:run({LIB_ROOT})