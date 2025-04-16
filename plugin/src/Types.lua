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

local ApiSubscribeResponse = t.interface({
	sessionId = t.string,
	messageCursor = t.number,
	messages = t.array(ApiSubscribeMessage),
})

local ApiSerializeResponse = t.interface({
	sessionId = t.string,
	modelContents = t.buffer,
})

local ApiRefPatchResponse = t.interface({
	sessionId = t.string,
	patch = ApiSubscribeMessage,
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
	ApiSubscribeResponse = ApiSubscribeResponse,
	ApiError = ApiError,

	ApiInstance = ApiInstance,
	ApiInstanceUpdate = ApiInstanceUpdate,
	ApiInstanceMetadata = ApiInstanceMetadata,
	ApiSubscribeMessage = ApiSubscribeMessage,
	ApiSerializeResponse = ApiSerializeResponse,
	ApiRefPatchResponse = ApiRefPatchResponse,
	ApiValue = ApiValue,
	RbxId = RbxId,

	-- Deprecated aliases during transition
	VirtualInstance = ApiInstance,
	VirtualMetadata = ApiInstanceMetadata,
	VirtualValue = ApiValue,
})
