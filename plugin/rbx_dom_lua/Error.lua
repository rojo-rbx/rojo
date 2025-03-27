local Error = {}
Error.__index = Error

Error.Kind = {
	UnknownProperty = "UnknownProperty",
	PropertyNotReadable = "PropertyNotReadable",
	PropertyNotWritable = "PropertyNotWritable",
	Roblox = "Roblox",
}

setmetatable(Error.Kind, {
	__index = function(_, key)
		error(("%q is not a valid member of Error.Kind"):format(tostring(key)), 2)
	end,
})

function Error.new(kind, extra)
	return setmetatable({
		kind = kind,
		extra = extra,
	}, Error)
end

function Error:__tostring()
	return ("Error(%s: %s)"):format(self.kind, tostring(self.extra))
end

return Error
