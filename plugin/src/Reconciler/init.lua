--[[
	This module defines the meat of the Rojo plugin and how it manages tracking
	and mutating the Roblox DOM.
]]

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Log = require(Packages.Log)

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
		__precommitCallbacks = {},
		__postcommitCallbacks = {},
	}

	return setmetatable(self, Reconciler)
end

function Reconciler:hookPrecommit(callback: (patch: any, instanceMap: any) -> ()): () -> ()
	table.insert(self.__precommitCallbacks, callback)
	Log.trace("Added precommit callback: {}", callback)

	return function()
		-- Remove the callback from the list
		for i, cb in self.__precommitCallbacks do
			if cb == callback then
				table.remove(self.__precommitCallbacks, i)
				Log.trace("Removed precommit callback: {}", callback)
				break
			end
		end
	end
end

function Reconciler:hookPostcommit(callback: (patch: any, instanceMap: any, unappliedPatch: any) -> ()): () -> ()
	table.insert(self.__postcommitCallbacks, callback)
	Log.trace("Added postcommit callback: {}", callback)

	return function()
		-- Remove the callback from the list
		for i, cb in self.__postcommitCallbacks do
			if cb == callback then
				table.remove(self.__postcommitCallbacks, i)
				Log.trace("Removed postcommit callback: {}", callback)
				break
			end
		end
	end
end

function Reconciler:applyPatch(patch)
	Timer.start("Reconciler:applyPatch")

	Timer.start("precommitCallbacks")
	-- Precommit callbacks must be serial in order to obey the contract that
	-- they execute before commit
	for _, callback in self.__precommitCallbacks do
		local success, err = pcall(callback, patch, self.__instanceMap)
		if not success then
			Log.warn("Precommit hook errored: {}", err)
		end
	end
	Timer.stop()

	Timer.start("apply")
	local unappliedPatch = applyPatch(self.__instanceMap, patch)
	Timer.stop()

	Timer.start("postcommitCallbacks")
	-- Postcommit callbacks can be called with spawn since regardless of firing order, they are
	-- guaranteed to be called after the commit
	for _, callback in self.__postcommitCallbacks do
		task.spawn(function()
			local success, err = pcall(callback, patch, self.__instanceMap, unappliedPatch)
			if not success then
				Log.warn("Postcommit hook errored: {}", err)
			end
		end)
	end
	Timer.stop()

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
