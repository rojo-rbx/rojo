--[[
	Defines the errors that can be returned by the reconciler.
]]

local Packages = script.Parent.Parent.Parent.Packages
local Fmt = require(Packages.Fmt)

local Error = {}

local function makeVariant(name)
	Error[name] = setmetatable({}, {
		__tostring = function()
			return "Error." .. name
		end,
	})
end

makeVariant("CannotCreateInstance")
makeVariant("CannotDecodeValue")
makeVariant("LackingPropertyPermissions")
makeVariant("OtherPropertyError")
makeVariant("RefDidNotExist")
makeVariant("UnknownProperty")
makeVariant("UnreadableProperty")
makeVariant("UnwritableProperty")

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
