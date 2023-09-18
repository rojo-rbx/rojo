local StudioService = game:GetService("StudioService")
local RunService = game:GetService("RunService")

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)
local Fmt = require(Packages.Fmt)
local t = require(Packages.t)
local Promise = require(Packages.Promise)

local ChangeBatcher = require(script.Parent.ChangeBatcher)
local encodePatchUpdate = require(script.Parent.ChangeBatcher.encodePatchUpdate)
local InstanceMap = require(script.Parent.InstanceMap)
local PatchSet = require(script.Parent.PatchSet)
local Reconciler = require(script.Parent.Reconciler)
local strict = require(script.Parent.strict)

local Status = strict("Session.Status", {
	NotStarted = "NotStarted",
	Connecting = "Connecting",
	Connected = "Connected",
	Disconnected = "Disconnected",
})

local function debugPatch(patch)
	return Fmt.debugify(patch, function(patch, output)
		output:writeLine("Patch {{")
		output:indent()

		for removed in ipairs(patch.removed) do
			output:writeLine("Remove ID {}", removed)
		end

		for id, added in pairs(patch.added) do
			output:writeLine("Add ID {} {:#?}", id, added)
		end

		for _, updated in ipairs(patch.updated) do
			output:writeLine("Update ID {} {:#?}", updated.id, updated)
		end

		output:unindent()
		output:write("}")
	end)
end

local ServeSession = {}
ServeSession.__index = ServeSession

ServeSession.Status = Status

local validateServeOptions = t.strictInterface({
	apiContext = t.table,
	openScriptsExternally = t.boolean,
	twoWaySync = t.boolean,
})

function ServeSession.new(options)
	assert(validateServeOptions(options))

	-- Declare self ahead of time to capture it in a closure
	local self
	local function onInstanceChanged(instance, propertyName)
		if not self.__twoWaySync then
			return
		end

		self.__changeBatcher:add(instance, propertyName)
	end

	local function onChangesFlushed(patch)
		self.__apiContext:write(patch)
	end

	local instanceMap = InstanceMap.new(onInstanceChanged)
	local changeBatcher = ChangeBatcher.new(instanceMap, onChangesFlushed)
	local reconciler = Reconciler.new(instanceMap)

	local connections = {}

	local connection = StudioService:GetPropertyChangedSignal("ActiveScript"):Connect(function()
		local activeScript = StudioService.ActiveScript

		if activeScript ~= nil then
			self:__onActiveScriptChanged(activeScript)
		end
	end)
	table.insert(connections, connection)

	self = {
		__status = Status.NotStarted,
		__apiContext = options.apiContext,
		__openScriptsExternally = options.openScriptsExternally,
		__twoWaySync = options.twoWaySync,
		__reconciler = reconciler,
		__instanceMap = instanceMap,
		__changeBatcher = changeBatcher,
		__statusChangedCallback = nil,
		__connections = connections,
	}

	setmetatable(self, ServeSession)

	return self
end

function ServeSession:__fmtDebug(output)
	output:writeLine("ServeSession {{")
	output:indent()

	output:writeLine("API Context: {:#?}", self.__apiContext)
	output:writeLine("Instances: {:#?}", self.__instanceMap)

	output:unindent()
	output:write("}")
end

function ServeSession:getStatus()
	return self.__status
end

function ServeSession:onStatusChanged(callback)
	self.__statusChangedCallback = callback
end

function ServeSession:setConfirmCallback(callback)
	self.__userConfirmCallback = callback
end

function ServeSession:hookPrecommit(callback)
	return self.__reconciler:hookPrecommit(callback)
end

function ServeSession:hookPostcommit(callback)
	return self.__reconciler:hookPostcommit(callback)
end

function ServeSession:start()
	self:__setStatus(Status.Connecting)

	self.__apiContext
		:connect()
		:andThen(function(serverInfo)
			self:__applyGameAndPlaceId(serverInfo)

			return self:__initialSync(serverInfo):andThen(function()
				self:__setStatus(Status.Connected, serverInfo.projectName)

				return self:__mainSyncLoop()
			end)
		end)
		:catch(function(err)
			if self.__status ~= Status.Disconnected then
				self:__stopInternal(err)
			end
		end)
end

function ServeSession:stop()
	self:__stopInternal()
end

function ServeSession:__applyGameAndPlaceId(serverInfo)
	if serverInfo.gameId ~= nil then
		game:SetUniverseId(serverInfo.gameId)
	end

	if serverInfo.placeId ~= nil then
		game:SetPlaceId(serverInfo.placeId)
	end
end

function ServeSession:__onActiveScriptChanged(activeScript)
	if not self.__openScriptsExternally then
		Log.trace("Not opening script {} because feature not enabled.", activeScript)

		return
	end

	if self.__status ~= Status.Connected then
		Log.trace("Not opening script {} because session is not connected.", activeScript)

		return
	end

	local scriptId = self.__instanceMap.fromInstances[activeScript]
	if scriptId == nil then
		Log.trace("Not opening script {} because it is not known by Rojo.", activeScript)

		return
	end

	Log.debug("Trying to open script {} externally...", activeScript)

	-- Force-close the script inside Studio... with a small delay in the middle
	-- to prevent Studio from crashing.
	spawn(function()
		local existingParent = activeScript.Parent
		activeScript.Parent = nil

		for i = 1, 3 do
			RunService.Heartbeat:Wait()
		end

		activeScript.Parent = existingParent
	end)

	-- Notify the Rojo server to open this script
	self.__apiContext:open(scriptId)
end

function ServeSession:__initialSync(serverInfo)
	return self.__apiContext:read({ serverInfo.rootInstanceId }):andThen(function(readResponseBody)
		-- Tell the API Context that we're up-to-date with the version of
		-- the tree defined in this response.
		self.__apiContext:setMessageCursor(readResponseBody.messageCursor)

		-- For any instances that line up with the Rojo server's view, start
		-- tracking them in the reconciler.
		Log.trace("Matching existing Roblox instances to Rojo IDs")
		self.__reconciler:hydrate(readResponseBody.instances, serverInfo.rootInstanceId, game)

		-- Calculate the initial patch to apply to the DataModel to catch us
		-- up to what Rojo thinks the place should look like.
		Log.trace("Computing changes that plugin needs to make to catch up to server...")
		local success, catchUpPatch =
			self.__reconciler:diff(readResponseBody.instances, serverInfo.rootInstanceId, game)

		if not success then
			Log.error("Could not compute a diff to catch up to the Rojo server: {:#?}", catchUpPatch)
		end

		for _, update in catchUpPatch.updated do
			if update.id == self.__instanceMap.fromInstances[game] and update.changedClassName ~= nil then
				-- Non-place projects will try to update the classname of game from DataModel to
				-- something like Folder, ModuleScript, etc. This would fail, so we exit with a clear
				-- message instead of crashing.
				return Promise.reject(
					"Cannot sync a model as a place."
						.. "\nEnsure Rojo is serving a project file that has a DataModel at the root of its tree and try again."
						.. "\nSee project file docs: https://rojo.space/docs/v7/project-format/"
				)
			end
		end

		Log.trace("Computed hydration patch: {:#?}", debugPatch(catchUpPatch))

		local userDecision = "Accept"
		if self.__userConfirmCallback ~= nil then
			userDecision = self.__userConfirmCallback(self.__instanceMap, catchUpPatch, serverInfo)
		end

		if userDecision == "Abort" then
			return Promise.reject("Aborted Rojo sync operation")
		elseif userDecision == "Reject" and self.__twoWaySync then
			-- The user wants their studio DOM to write back to their Rojo DOM
			-- so we will reverse the patch and send it back

			local inversePatch = PatchSet.newEmpty()

			-- Send back the current properties
			for _, change in catchUpPatch.updated do
				local instance = self.__instanceMap.fromIds[change.id]
				if not instance then
					continue
				end

				local update = encodePatchUpdate(instance, change.id, change.changedProperties)
				table.insert(inversePatch.updated, update)
			end
			-- Add the removed instances back to Rojo
			-- selene:allow(empty_if, unused_variable)
			for _, instance in catchUpPatch.removed do
				-- TODO: Generate ID for our instance and add it to inversePatch.added
			end
			-- Remove the additions we've rejected
			for id, _change in catchUpPatch.added do
				table.insert(inversePatch.removed, id)
			end

			self.__apiContext:write(inversePatch)
		elseif userDecision == "Accept" then
			local unappliedPatch = self.__reconciler:applyPatch(catchUpPatch)

			if not PatchSet.isEmpty(unappliedPatch) then
				Log.warn(
					"Could not apply all changes requested by the Rojo server:\n{}",
					PatchSet.humanSummary(self.__instanceMap, unappliedPatch)
				)
			end
		end
	end)
end

function ServeSession:__mainSyncLoop()
	return Promise.new(function(resolve, reject)
		while self.__status == Status.Connected do
			local success, result = self.__apiContext
				:retrieveMessages()
				:andThen(function(messages)
					if self.__status == Status.Disconnected then
						-- In the time it took to retrieve messages, we disconnected
						-- so we just resolve immediately without patching anything
						return
					end

					Log.trace("Serve session {} retrieved {} messages", tostring(self), #messages)

					for _, message in messages do
						local unappliedPatch = self.__reconciler:applyPatch(message)

						if not PatchSet.isEmpty(unappliedPatch) then
							Log.warn(
								"Could not apply all changes requested by the Rojo server:\n{}",
								PatchSet.humanSummary(self.__instanceMap, unappliedPatch)
							)
						end
					end
				end)
				:await()

			if self.__status == Status.Disconnected then
				-- If we are no longer connected after applying, we stop silently
				-- without checking for errors as they are no longer relevant
				break
			elseif success == false then
				reject(result)
			end
		end

		-- We are no longer connected, so we resolve the promise
		resolve()
	end)
end

function ServeSession:__stopInternal(err)
	self:__setStatus(Status.Disconnected, err)
	self.__apiContext:disconnect()
	self.__instanceMap:stop()
	self.__changeBatcher:stop()

	for _, connection in ipairs(self.__connections) do
		connection:Disconnect()
	end
	self.__connections = {}
end

function ServeSession:__setStatus(status, detail)
	self.__status = status

	if self.__statusChangedCallback ~= nil then
		self.__statusChangedCallback(status, detail)
	end
end

return ServeSession
