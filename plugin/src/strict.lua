local function strictInner(name, target)
	assert(type(name) == "string", "Argument #1 to `strict` must be a string or the table to modify")
	assert(type(target) == "table", "Argument #2 to `strict` must be nil or the table to modify")

	setmetatable(target, {
		__index = function(_, key)
			error(("%q is not a valid member of strict table %q"):format(tostring(key), name), 2)
		end,

		__newindex = function()
			error(("Strict table %q is read-only"):format(name), 2)
		end,
	})

	return target
end

return function(nameOrTarget, target)
	if type(nameOrTarget) == "string" then
		return strictInner(nameOrTarget, target)
	else
		return strictInner("<unnamed table>", target)
	end
end
