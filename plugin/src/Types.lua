local t = require(script.Parent.Parent.t)

local DevSettings = require(script.Parent.DevSettings)

local VirtualValue = t.interface({
	Type = t.string,
	Value = t.optional(t.any),
})

local VirtualMetadata = t.interface({
	ignoreUnknownInstances = t.optional(t.boolean),
})

local VirtualInstance = t.interface({
	Name = t.string,
	ClassName = t.string,
	Properties = t.map(t.string, VirtualValue),
	Metadata = t.optional(VirtualMetadata)
})

local function ifEnabled(innerCheck)
	return function(...)
		if DevSettings:shouldTypecheck() then
			return innerCheck(...)
		else
			return true
		end
	end
end

return {
	ifEnabled = ifEnabled,
	VirtualInstance = VirtualInstance,
	VirtualMetadata = VirtualMetadata,
	VirtualValue = VirtualValue,
}