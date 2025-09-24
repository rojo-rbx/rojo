local ReplicatedStorage = game:GetService("ReplicatedStorage")

local TestEZ = require(ReplicatedStorage.Packages:WaitForChild("TestEZ", 10))

local Rojo = ReplicatedStorage.Rojo

local Settings = require(Rojo.Plugin.Settings)
Settings:set("logLevel", "Trace")
Settings:set("typecheckingEnabled", true)

local results = require(Rojo.Plugin.runTests)(TestEZ)

-- Roblox's Luau execution gets mad about cyclical tables.
-- Rather than making TestEZ not do that, we just send back the important info.
return {
	failureCount = results.failureCount,
	successCount = results.successCount,
	skippedCount = results.skippedCount,
}
