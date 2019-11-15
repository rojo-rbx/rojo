local function writeFmt(buffer, template, ...)
	local currentArg = 0
	local i = 1
	local len = #template

	while i < len do
		local openBrace = template:find("{", i)

		if openBrace == nil then
			buffer:writeRaw(template:sub(i))
			break
		else
			if openBrace - i > 0 then
				buffer:writeRaw(template:sub(i, openBrace - 1))
			end

			local charAfterBrace = template:sub(openBrace + 1, openBrace + 1)
			if charAfterBrace == "{" then
				buffer:writeRaw(template:sub(i, openBrace))
				i = openBrace + 2
			else
				local closeBrace = template:find("}", openBrace + 1)
				assert(closeBrace ~= nil, "Unclosed formatting specifier. Use '{{' to write an open brace.")

				local formatSpecifier = template:sub(openBrace + 1, closeBrace - 1)

				currentArg = currentArg + 1

				if formatSpecifier == "" then
					local arg = select(currentArg, ...)
					buffer:writeRaw(tostring(arg))
				else
					error("unsupported format specifier " .. formatSpecifier, 2)
				end

				i = closeBrace + 1
			end
		end
	end
end

local function writeLineFmt(buffer, template, ...)
	writeFmt(buffer, template, ...)
	table.insert(buffer, "\n")
end

local function debugOutputBuffer()
	local buffer = {}
	local indentLevel = 0
	local indentation = ""

	function buffer:writeLine(template, ...)
		return writeLineFmt(self, template, ...)
	end

	function buffer:write(template, ...)
		return writeFmt(self, template, ...)
	end

	function buffer:writeRaw(value)
		if #indentation > 0 then
			value = value:gsub("\n", "\n" .. indentation)
		end

		table.insert(self, value)
	end

	function buffer:writeLineRaw(piece)
		if #indentation > 0 then
			self:writeRaw(indentation)
		end

		self:writeRaw(piece)
		table.insert(self, "\n")
	end

	function buffer:push(template, ...)
		local value = string.format(template, ...)

		self:writeLineRaw(value)
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
		return table.concat(self, "")
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
	writeFmt = writeFmt,
	writeLineFmt = writeLineFmt,
}