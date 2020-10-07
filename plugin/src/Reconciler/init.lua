--[[
	This module defines the meat of the Rojo plugin and how it manages tracking
	and mutating the Roblox DOM.
]]

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
	return applyPatch(self.__instanceMap, patch)
end

function Reconciler:hydrate(virtualInstances, rootId, rootInstance)
	return hydrate(self.__instanceMap, virtualInstances, rootId, rootInstance)
end

function Reconciler:diff(virtualInstances, rootId)
	return diff(self.__instanceMap, virtualInstances, rootId)
end

return Reconciler