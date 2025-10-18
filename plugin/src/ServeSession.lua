local StudioService = game:GetService("StudioService")
local RunService = game:GetService("RunService")
local ChangeHistoryService = game:GetService("ChangeHistoryService")
local SerializationService = game:GetService("SerializationService")
local Selection = game:GetService("Selection")

local Packages = script.Parent.Parent.Packages
local Log = require(Packages.Log)
local Fmt = require(Packages.Fmt)
local t = require(Packages.t)
local Promise = require(Packages.Promise)
local Timer = require(script.Parent.Timer)

local ChangeBatcher = require(script.Parent.ChangeBatcher)
local encodePatchUpdate = require(script.Parent.ChangeBatcher.encodePatchUpdate)
local InstanceMap = require(script.Parent.InstanceMap)
local PatchSet = require(script.Parent.PatchSet)
local Reconciler = require(script.Parent.Reconciler)
local strict = require(script.Parent.strict)
local Settings = require(script.Parent.Settings)

local Status = strict("Session.Status", {
	NotStarted = "NotStarted",
	Connecting = "Connecting",
	Connected = "Connected",
	Disconnected = "Disconnected",
})

local function debugPatch(object)
	return Fmt.debugify(object, function(patch, output)
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

local function attemptReparent(instance, parent)
	return pcall(function()
		instance.Parent = parent
	end)
end

local ServeSession = {}
ServeSession.__index = ServeSession

ServeSession.Status = Status

local validateServeOptions = t.strictInterface({
	apiContext = t.table,
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
		__twoWaySync = options.twoWaySync,
		__reconciler = reconciler,
		__instanceMap = instanceMap,
		__changeBatcher = changeBatcher,
		__statusChangedCallback = nil,
		__connections = connections,
		__precommitCallbacks = {},
		__postcommitCallbacks = {},
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

--[=[
	Hooks a function to run before patch application.
	The provided function is called with the incoming patch and an InstanceMap
	as parameters.
]=]
function ServeSession:hookPrecommit(callback)
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

--[=[
	Hooks a function to run after patch application.
	The provided function is called with the applied patch, the current
	InstanceMap, and a PatchSet containing any unapplied changes.
]=]
function ServeSession:hookPostcommit(callback)
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

function ServeSession:start()
	self:__setStatus(Status.Connecting)

	self.__apiContext
		:connect()
		:andThen(function(serverInfo)
			return self:__initialSync(serverInfo):andThen(function()
				self:__setStatus(Status.Connected, serverInfo.projectName)
				self:__applyGameAndPlaceId(serverInfo)

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
	if not Settings:get("openScriptsExternally") then
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

		for _ = 1, 3 do
			RunService.Heartbeat:Wait()
		end

		activeScript.Parent = existingParent
	end)

	-- Notify the Rojo server to open this script
	self.__apiContext:open(scriptId)
end

function ServeSession:__replaceInstances(idList)
	if #idList == 0 then
		return true, PatchSet.newEmpty()
	end
	-- It would be annoying if selection went away, so we try to preserve it.
	local selection = Selection:Get()
	local selectionMap = {}
	for i, instance in selection do
		selectionMap[instance] = i
	end

	-- TODO: Should we do this in multiple requests so we can more granularly mark failures?
	local modelSuccess, replacements = self.__apiContext
		:serialize(idList)
		:andThen(function(response)
			Log.debug("Deserializing results from serialize endpoint")
			local objects = SerializationService:DeserializeInstancesAsync(response.modelContents)
			if not objects[1] then
				return Promise.reject("Serialize endpoint did not deserialize into any Instances")
			end
			if #objects[1]:GetChildren() ~= #idList then
				return Promise.reject("Serialize endpoint did not return the correct number of Instances")
			end

			local instanceMap = {}
			for _, item in objects[1]:GetChildren() do
				instanceMap[item.Name] = item.Value
			end
			return instanceMap
		end)
		:await()

	local refSuccess, refPatch = self.__apiContext
		:refPatch(idList)
		:andThen(function(response)
			return response.patch
		end)
		:await()

	if not (modelSuccess and refSuccess) then
		return false
	end

	for id, replacement in replacements do
		local oldInstance = self.__instanceMap.fromIds[id]
		if not oldInstance then
			-- TODO: Why would this happen?
			Log.warn("Instance {} not found in InstanceMap during sync replacement", id)
			continue
		end

		self.__instanceMap:insert(id, replacement)
		Log.trace("Swapping Instance {} out via api/models/ endpoint", id)
		local oldParent = oldInstance.Parent
		for _, child in oldInstance:GetChildren() do
			-- Some children cannot be reparented, such as a TouchTransmitter
			local reparentSuccess, reparentError = attemptReparent(child, replacement)
			if not reparentSuccess then
				Log.warn(
					"Could not reparent child {} of instance {} during sync replacement: {}",
					child.Name,
					oldInstance.Name,
					reparentError
				)
			end
		end

		-- ChangeHistoryService doesn't like it if an Instance has been
		-- Destroyed. So, we have to accept the potential memory hit and
		-- just set the parent to `nil`.
		local deleteSuccess, deleteError = attemptReparent(oldInstance, nil)
		local replaceSuccess, replaceError = attemptReparent(replacement, oldParent)

		if not (deleteSuccess and replaceSuccess) then
			Log.warn(
				"Could not swap instances {} and {} during sync replacement: {}",
				oldInstance.Name,
				replacement.Name,
				(deleteError or "") .. "\n" .. (replaceError or "")
			)

			-- We need to revert the failed swap to avoid losing the old instance and children.
			for _, child in replacement:GetChildren() do
				attemptReparent(child, oldInstance)
			end
			attemptReparent(oldInstance, oldParent)

			-- Our replacement should never have existed in the first place, so we can just destroy it.
			replacement:Destroy()
			continue
		end

		if selectionMap[oldInstance] then
			-- This is a bit funky, but it saves the order of Selection
			-- which might matter for some use cases.
			selection[selectionMap[oldInstance]] = replacement
		end
	end

	local patchApplySuccess, unappliedPatch = pcall(self.__reconciler.applyPatch, self.__reconciler, refPatch)
	if patchApplySuccess then
		Selection:Set(selection)
		return true, unappliedPatch
	else
		error(unappliedPatch)
	end
end

function ServeSession:__applyPatch(patch)
	local patchTimestamp = DateTime.now():FormatLocalTime("LTS", "en-us")
	local historyRecording = ChangeHistoryService:TryBeginRecording("Rojo: Patch " .. patchTimestamp)
	if not historyRecording then
		-- There can only be one recording at a time
		Log.debug("Failed to begin history recording for " .. patchTimestamp .. ". Another recording is in progress.")
	end

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

	local patchApplySuccess, unappliedPatch = pcall(self.__reconciler.applyPatch, self.__reconciler, patch)
	if not patchApplySuccess then
		if historyRecording then
			ChangeHistoryService:FinishRecording(historyRecording, Enum.FinishRecordingOperation.Commit)
		end
		-- This might make a weird stack trace but the only way applyPatch can
		-- fail is if a bug occurs so it's probably fine.
		error(unappliedPatch)
	end

	if Settings:get("enableSyncFallback") and not PatchSet.isEmpty(unappliedPatch) then
		-- Some changes did not apply, let's try replacing them instead
		local addedIdList = PatchSet.addedIdList(unappliedPatch)
		local updatedIdList = PatchSet.updatedIdList(unappliedPatch)

		Log.debug("ServeSession:__replaceInstances(unappliedPatch.added)")
		Timer.start("ServeSession:__replaceInstances(unappliedPatch.added)")
		local addSuccess, unappliedAddedRefs = self:__replaceInstances(addedIdList)
		Timer.stop()

		Log.debug("ServeSession:__replaceInstances(unappliedPatch.updated)")
		Timer.start("ServeSession:__replaceInstances(unappliedPatch.updated)")
		local updateSuccess, unappliedUpdateRefs = self:__replaceInstances(updatedIdList)
		Timer.stop()

		-- Update the unapplied patch to reflect which Instances were replaced successfully
		if addSuccess then
			table.clear(unappliedPatch.added)
			PatchSet.assign(unappliedPatch, unappliedAddedRefs)
		end
		if updateSuccess then
			table.clear(unappliedPatch.updated)
			PatchSet.assign(unappliedPatch, unappliedUpdateRefs)
		end
	end

	if not PatchSet.isEmpty(unappliedPatch) then
		Log.debug(
			"Could not apply all changes requested by the Rojo server:\n{}",
			PatchSet.humanSummary(self.__instanceMap, unappliedPatch)
		)
	end

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

	if historyRecording then
		ChangeHistoryService:FinishRecording(historyRecording, Enum.FinishRecordingOperation.Commit)
	end
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
		elseif userDecision == "Reject" then
			if not self.__twoWaySync then
				return Promise.reject("Cannot reject sync operation without two-way sync enabled")
			end
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
			-- selene:allow(empty_if, unused_variable, empty_loop)
			for _, instance in catchUpPatch.removed do
				-- TODO: Generate ID for our instance and add it to inversePatch.added
			end
			-- Remove the additions we've rejected
			for id, _change in catchUpPatch.added do
				table.insert(inversePatch.removed, id)
			end

			return self.__apiContext:write(inversePatch)
		elseif userDecision == "Accept" then
			self:__applyPatch(catchUpPatch)
			return Promise.resolve()
		else
			return Promise.reject("Invalid user decision: " .. userDecision)
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
						self:__applyPatch(message)
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
