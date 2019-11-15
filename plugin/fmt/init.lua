local function debugOutputBuffer()
	local buffer = {}
	local indentLevel = 0
	local indentation = ""

	function buffer:push(template, ...)
		local value = string.format(template, ...)

		if #indentation > 0 then
			value = indentation .. value:gsub("\n", "\n" .. indentation)
		end

		table.insert(self, value)
	end

	function buffer:indent()
		indentLevel = indentLevel + 1
		indentation = string.rep("    ", indentLevel)
	end

	function buffer:unindent()
		indentLevel = math.max(0, indentLevel - 1)
		indentation = string.rep("    ", indentLevel)
	end

	function buffer:finish()
		return table.concat(self, "\n")
	end

	return buffer
end

local function debugInner(value)
	local valueType = typeof(value)

	if valueType == "string" then
		return string.format("%q", value)
	elseif valueType == "number" then
		return tostring(value)
	elseif valueType == "table" then
		local debugImpl = getmetatable(value).__fmtDebug

		if debugImpl ~= nil then
			return debugImpl()
		else
			-- TODO: Nicer default debug implementation?
			return tostring(value)
		end
	end
end

return {
	debugOutputBuffer = debugOutputBuffer,
}