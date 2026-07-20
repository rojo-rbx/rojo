local Packages = script.Parent.Parent.Packages
local t = require(Packages.t)
local Settings = require(script.Parent.Settings)
local strict = require(script.Parent.strict)

local RbxId = t.string

local ApiValue = t.keys(t.string)

local ApiInstanceMetadata = t.interface({
	ignoreUnknownInstances = t.optional(t.boolean),
})

local ApiInstance = t.interface({
	Id = RbxId,
	Parent = t.optional(RbxId),
	Name = t.string,
	ClassName = t.string,
	Properties = t.map(t.string, ApiValue),
	Metadata = t.optional(ApiInstanceMetadata),
	Children = t.array(RbxId),
})

local ApiInstanceUpdate = t.interface({
	id = RbxId,
	changedName = t.optional(t.string),
	changedClassName = t.optional(t.string),
	changedProperties = t.map(t.string, ApiValue),
	changedMetadata = t.optional(ApiInstanceMetadata),
})

local ApiSubscribeMessage = t.interface({
	removed = t.array(RbxId),
	added = t.map(RbxId, ApiInstance),
	updated = t.array(ApiInstanceUpdate),
})

local ApiInfoResponse = t.interface({
	sessionId = t.string,
	serverVersion = t.string,
	protocolVersion = t.number,
	expectedPlaceIds = t.optional(t.array(t.number)),
	rootInstanceId = RbxId,
})

local ApiReadResponse = t.interface({
	sessionId = t.string,
	messageCursor = t.number,
	instances = t.map(RbxId, ApiInstance),
})

local SocketPacketType = t.union(t.literal("messages"))

local MessagesPacket = t.interface({
	messageCursor = t.number,
	messages = t.array(ApiSubscribeMessage),
})

local SocketPacketBody = t.union(MessagesPacket)

local ApiSocketPacket = t.interface({
	sessionId = t.string,
	packetType = SocketPacketType,
	body = SocketPacketBody,
})

local ApiSerializeResponse = t.interface({
	sessionId = t.string,
	modelContents = t.buffer,
})

local ApiRefPatchResponse = t.interface({
	sessionId = t.string,
	patch = ApiSubscribeMessage,
})

local function Uuid(value)
	if type(value) ~= "string" then
		return false, string.format("string expected, got %s", type(value))
	end

	local first, second, third, fourth, fifth =
		value:match("^([0-9a-fA-F]+)%-([0-9a-fA-F]+)%-([0-9a-fA-F]+)%-([0-9a-fA-F]+)%-([0-9a-fA-F]+)$")
	if first == nil or #first ~= 8 or #second ~= 4 or #third ~= 4 or #fourth ~= 4 or #fifth ~= 12 then
		return false, "UUID string expected"
	end

	return true
end

local ExecJobState = t.union(
	t.literal("pending"),
	t.literal("claimed"),
	t.literal("succeeded"),
	t.literal("failed"),
	t.literal("timedOut")
)

local StudioMode = t.union(t.literal("edit"), t.literal("play"), t.literal("run"), t.literal("unknown"))

local AutomationRegistration = t.union(t.literal("registered"), t.literal("refreshed"), t.literal("conflict"))

local ApiAutomationHeartbeatResponse = t.strictInterface({
	registration = AutomationRegistration,
	activePluginSessionId = t.optional(Uuid),
})

local AutomationJobState = t.union(
	t.literal("pending"),
	t.literal("claimed"),
	t.literal("succeeded"),
	t.literal("failed"),
	t.literal("timedOut")
)

local InspectTarget = t.strictInterface({
	kind = t.literal("path"),
	segments = t.array(t.string),
})

local InspectRequest = t.strictInterface({
	kind = t.literal("inspect"),
	target = InspectTarget,
	depth = t.number,
	maxChildren = t.number,
	maxInstances = t.number,
	includeProperties = t.boolean,
	includeAttributes = t.boolean,
	includeTags = t.boolean,
})

local ApiAutomationClaimResponse = t.strictInterface({
	jobId = Uuid,
	state = t.literal("claimed"),
	request = InspectRequest,
	executionTimeoutMs = t.number,
})

local ApiAutomationJobResponse = t.strictInterface({
	jobId = Uuid,
	state = AutomationJobState,
	claimedByPluginSessionId = t.optional(Uuid),
	result = t.optional(t.table),
	error = t.optional(t.string),
})

local ExecLogLevel = t.union(t.literal("print"), t.literal("warn"))

local ExecLog = t.strictInterface({
	level = ExecLogLevel,
	message = t.string,
})

local function ExecValue(value, seen)
	if type(value) ~= "table" then
		return false, string.format("exec result table expected, got %s", type(value))
	end

	seen = seen or {}
	if seen[value] then
		return false, "cyclic exec result value"
	end
	seen[value] = true

	local kind = value.kind
	local valid, message
	if kind == "nil" then
		valid, message = t.strictInterface({ kind = t.literal("nil") })(value)
	elseif kind == "string" then
		valid, message = t.strictInterface({ kind = t.literal("string"), value = t.string })(value)
	elseif kind == "number" then
		valid, message = t.strictInterface({ kind = t.literal("number"), value = t.number })(value)
	elseif kind == "boolean" then
		valid, message = t.strictInterface({ kind = t.literal("boolean"), value = t.boolean })(value)
	elseif kind == "array" then
		valid, message = t.strictInterface({ kind = t.literal("array"), value = t.table })(value)
		if valid then
			valid, message = t.array(function(child)
				return ExecValue(child, seen)
			end)(value.value)
		end
	elseif kind == "table" then
		valid, message = t.strictInterface({ kind = t.literal("table"), value = t.table })(value)
		if valid then
			valid, message = t.array(function(entry)
				local entryValid, entryMessage = t.strictInterface({
					key = t.string,
					value = t.table,
				})(entry)
				if not entryValid then
					return false, entryMessage
				end
				return ExecValue(entry.value, seen)
			end)(value.value)
		end
	else
		valid, message = false, string.format("unknown exec result kind %q", tostring(kind))
	end

	seen[value] = nil
	return valid, message
end

local ApiExecClaimResponse = t.strictInterface({
	jobId = Uuid,
	scriptName = t.string,
	source = t.string,
	state = t.literal("claimed"),
})

local ApiExecJobResponse = t.strictInterface({
	jobId = Uuid,
	scriptName = t.string,
	state = ExecJobState,
	result = t.optional(ExecValue),
	logs = t.optional(t.array(ExecLog)),
	error = t.optional(t.string),
	traceback = t.optional(t.string),
})

local ApiError = t.interface({
	kind = t.union(t.literal("NotFound"), t.literal("BadRequest"), t.literal("InternalError")),
	details = t.string,
})

local function ifEnabled(innerCheck)
	return function(...)
		if Settings:get("typecheckingEnabled") then
			return innerCheck(...)
		else
			return true
		end
	end
end

return strict("Types", {
	ifEnabled = ifEnabled,

	ApiInfoResponse = ApiInfoResponse,
	ApiReadResponse = ApiReadResponse,
	ApiSocketPacket = ApiSocketPacket,
	ApiError = ApiError,

	ApiInstance = ApiInstance,
	ApiInstanceUpdate = ApiInstanceUpdate,
	ApiInstanceMetadata = ApiInstanceMetadata,
	ApiSubscribeMessage = ApiSubscribeMessage,
	ApiSerializeResponse = ApiSerializeResponse,
	ApiRefPatchResponse = ApiRefPatchResponse,
	ApiExecClaimResponse = ApiExecClaimResponse,
	ApiExecJobResponse = ApiExecJobResponse,
	ApiAutomationHeartbeatResponse = ApiAutomationHeartbeatResponse,
	ApiAutomationClaimResponse = ApiAutomationClaimResponse,
	ApiAutomationJobResponse = ApiAutomationJobResponse,
	AutomationJobState = AutomationJobState,
	AutomationRegistration = AutomationRegistration,
	StudioMode = StudioMode,
	ExecJobState = ExecJobState,
	ExecValue = ExecValue,
	ExecLog = ExecLog,
	ExecLogLevel = ExecLogLevel,
	Uuid = Uuid,
	ApiValue = ApiValue,
	RbxId = RbxId,

	-- Deprecated aliases during transition
	VirtualInstance = ApiInstance,
	VirtualMetadata = ApiInstanceMetadata,
	VirtualValue = ApiValue,
})
