--[[
	This module defines the meat of the Rojo plugin and how it manages tracking
	and mutating the Roblox DOM.
]]

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Timer = require(Plugin.Timer)

local applyPatch = require(script.applyPatch)
local hydrate = require(script.hydrate)
local diff = require(script.diff)

local Reconciler = {}
Reconciler.__index = Reconciler

function Reconciler.new(instanceMap)
	local self = {
		-- Tracks all of the instances known by the reconciler by ID.
		__instanceMap = instanceMap,
	}

	return setmetatable(self, Reconciler)
end

function Reconciler:applyPatch(patch)
	Timer.start("Reconciler:applyPatch")

	local unappliedPatch = applyPatch(self.__instanceMap, patch)

	Timer.stop()
	return unappliedPatch
end

function Reconciler:hydrate(virtualInstances, rootId, rootInstance)
	Timer.start("Reconciler:hydrate")
	local result = hydrate(self.__instanceMap, virtualInstances, rootId, rootInstance)
	Timer.stop()

	return result
end

function Reconciler:diff(virtualInstances, rootId)
	Timer.start("Reconciler:diff")
	local success, result = diff(self.__instanceMap, virtualInstances, rootId)
	Timer.stop()

	return success, result
end

return Reconciler
