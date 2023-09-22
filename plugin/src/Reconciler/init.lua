--[[
	This module defines the meat of the Rojo plugin and how it manages tracking
	and mutating the Roblox DOM.
]]
local ChangeHistoryService = game:GetService("ChangeHistoryService")

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)

local PatchSet = require(script.Parent.PatchSet)

local fetchInstances = require(script.fetchInstances)
local applyPatch = require(script.applyPatch)
local hydrate = require(script.hydrate)
local diff = require(script.diff)

local Reconciler = {}
Reconciler.__index = Reconciler

function Reconciler.new(instanceMap, apiContext, fetchOnPatchFail: boolean)
	local self = {
		-- Tracks all of the instances known by the reconciler by ID.
		__instanceMap = instanceMap,
		-- An API context for sending requests back to the server
		__apiContext = apiContext,
		__fetchOnPatchFail = fetchOnPatchFail,
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
	for _, callback in self.__precommitCallbacks do
		local success, err = pcall(callback, patch, self.__instanceMap)
		if not success then
			Log.warn("Precommit hook errored: {}", err)
		end
	end

	local patchTimestamp = DateTime.now():FormatLocalTime("LTS", "en-us")

	local unappliedPatch = applyPatch(self.__instanceMap, patch)

	if self.__fetchOnPatchFail then
		-- TODO Is it worth doing this for additions that fail?
		-- It seems unlikely that a valid Instance can't be made with `Instance.new`
		-- but can be made using GetObjects
		if PatchSet.hasUpdates(unappliedPatch) then
			local idList = table.create(#unappliedPatch.updated)
			for i, entry in unappliedPatch.updated do
				idList[i] = entry.id
			end
			-- TODO this is destructive to any properties that are
			-- overwritten by the user but not known to Rojo. We can probably
			-- mitigate that by keeping tabs of any instances we need to swap.
			fetchInstances(idList, self.__instanceMap, self.__apiContext)
			table.clear(unappliedPatch.updated)
		end
	end

	ChangeHistoryService:SetWaypoint("Rojo: Patch " .. patchTimestamp)

	for _, callback in self.__postcommitCallbacks do
		local success, err = pcall(callback, patch, self.__instanceMap, unappliedPatch)
		if not success then
			Log.warn("Postcommit hook errored: {}", err)
		end
	end

	return unappliedPatch
end

function Reconciler:hydrate(virtualInstances, rootId, rootInstance)
	return hydrate(self.__instanceMap, virtualInstances, rootId, rootInstance)
end

function Reconciler:diff(virtualInstances, rootId)
	return diff(self.__instanceMap, virtualInstances, rootId)
end

return Reconciler
