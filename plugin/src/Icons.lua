local Icons = {
	StartSession = "",
	SessionActive = "",
	Configure = "",
}

setmetatable(Icons, {
	__index = function(_, key)
		error(("%q is not a valid member of Icons"):format(tostring(key)), 2)
	end
})

return Icons