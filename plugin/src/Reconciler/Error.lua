--[[
	Defines the errors that can be returned by the reconciler.
]]

local Fmt = require(script.Parent.Parent.Parent.Fmt)

local Error = {}

local function makeVariant(name)
	Error[name] = setmetatable({}, {
		__tostring = function()
			return "Error." .. name
		end,
	})
end

makeVariant("CannotCreateInstance")
makeVariant("UnwritableProperty")
makeVariant("LackingPropertyPermissions")
makeVariant("OtherPropertyError")
makeVariant("CannotDecodeValue")

function Error.new(kind, details)
	return setmetatable({
		kind = kind,
		details = details,
	}, Error)
end

function Error:__tostring()
	return Fmt.fmt("Error({}): {:#?}", self.kind, self.details)
end

return Error