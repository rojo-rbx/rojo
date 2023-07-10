--[[
	This module defines the meat of the Rojo plugin and how it manages tracking
	and mutating the Roblox DOM.
]]

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)

local applyPatch = require(script.applyPatch)
local hydrate = require(script.hydrate)
local diff = require(script.diff)

local Reconciler = {}
Reconciler.__index = Reconciler

function Reconciler.new(instanceMap)
	local self = {
		-- Tracks all of the instances known by the reconciler by ID.
		__instanceMap = instanceMap,
		__precommitCallbacks = {},
	}

	return setmetatable(self, Reconciler)
end

function Reconciler:hookPrecommit(callback: (patch: any, instanceMap: any) -> ()): () -> ()
	table.insert(self.__precommitCallbacks, callback)

	return function()
		-- Remove the callback from the list
		for i, cb in self.__precommitCallbacks do
			if cb == callback then
				table.remove(self.__precommitCallbacks, i)
				break
			end
		end
	end
end

function Reconciler:applyPatch(patch)
	for _, callback in self.__precommitCallbacks do
		local success, err = pcall(callback, patch, self.__instanceMap)
		if not success then
			Log.warn("Precommit hook errored: {}", err)
		end
	end

	return applyPatch(self.__instanceMap, patch)
end

function Reconciler:hydrate(virtualInstances, rootId, rootInstance)
	return hydrate(self.__instanceMap, virtualInstances, rootId, rootInstance)
end

function Reconciler:diff(virtualInstances, rootId)
	return diff(self.__instanceMap, virtualInstances, rootId)
end

return Reconciler
