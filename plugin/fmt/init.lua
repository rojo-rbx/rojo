--[[
	This library describes a formatting mechanism akin to Rust's std::fmt.

	It has a couple building blocks:

	* A new syntax for formatting strings, taken verbatim from Rust. It'd also
	  be possible to use printf-style formatting specifiers to integrate with
	  the existing string.format utility.

	* An equivalent to Rust's `Display` trait. We're mapping the semantics of
	  tostring and the __tostring metamethod onto this trait. A lot of types
	  should already have __tostring implementations, too!

	* An equivalent to Rust's `Debug` trait. This library Lua-ifies that idea by
	  inventing a new metamethod, `__fmtDebug`. We pass along the "extended
	  form" attribute which is the equivalent of the "alternate mode" in Rust's
	  Debug trait since it's the author's opinion that treating it as a
	  verbosity flag is semantically accurate.
]]

--[[
	The default implementation of __fmtDebug for tables when the extended option
	is not set.
]]
local function defaultTableDebug(buffer, input)
	buffer:writeRaw("{")

	for key, value in pairs(input) do
		buffer:write("[{:?}] = {:?}", key, value)

		if next(input, key) ~= nil then
			buffer:writeRaw(", ")
		end
	end

	buffer:writeRaw("}")
end

--[[
	The default implementation of __fmtDebug for tables with the extended option
	set.
]]
local function defaultTableDebugExtended(buffer, input)
	-- Special case for empty tables.
	if next(input) == nil then
		buffer:writeRaw("{}")
		return
	end

	buffer:writeLineRaw("{")
	buffer:indent()

	for key, value in pairs(input) do
		buffer:writeLine("[{:?}] = {:#?},", key, value)
	end

	buffer:unindent()
	buffer:writeRaw("}")
end

--[[
	The default debug representation for all types.
]]
local function debugImpl(buffer, value, extendedForm)
	local valueType = typeof(value)

	if valueType == "string" then
		local formatted = string.format("%q", value)
		buffer:writeRaw(formatted)
	elseif valueType == "table" then
		local valueMeta = getmetatable(value)

		if valueMeta ~= nil and  valueMeta.__fmtDebug ~= nil then
			-- This type implement's the metamethod we made up to line up with
			-- Rust's 'Debug' trait.

			valueMeta.__fmtDebug(value, buffer, extendedForm)
		else
			if extendedForm then
				defaultTableDebugExtended(buffer, value)
			else
				defaultTableDebug(buffer, value)
			end
		end
	elseif valueType == "Instance" then
		buffer:writeRaw(value:GetFullName())
	else
		buffer:writeRaw(tostring(value))
	end
end

--[[
	Defines and implements the library's template syntax.
]]
local function writeFmt(buffer, template, ...)
	local currentArg = 0
	local i = 1
	local len = #template

	while i <= len do
		local openBrace = template:find("{", i)

		if openBrace == nil then
			-- There are no remaining open braces in this string, so we can
			-- write the rest of the string to the buffer.

			buffer:writeRaw(template:sub(i))
			break
		else
			-- We found an open brace! This could be:
			-- - A literal '{', written as '{{'
			-- - The beginning of an interpolation, like '{}'
			-- - An error, if there's no matching '}'

			local charAfterBrace = template:sub(openBrace + 1, openBrace + 1)
			if charAfterBrace == "{" then
				-- This is a literal brace, so we'll write everything up to this
				-- point (including the first brace), and then skip over the
				-- second brace.

				buffer:writeRaw(template:sub(i, openBrace))
				i = openBrace + 2
			else
				-- This SHOULD be an interpolation. We'll find our matching
				-- brace and treat the contents as the formatting specifier.

				-- If there were any unwritten characters before this
				-- interpolation, write them to the buffer.
				if openBrace - i > 0 then
					buffer:writeRaw(template:sub(i, openBrace - 1))
				end

				local closeBrace = template:find("}", openBrace + 1)
				assert(closeBrace ~= nil, "Unclosed formatting specifier. Use '{{' to write an open brace.")

				local formatSpecifier = template:sub(openBrace + 1, closeBrace - 1)
				currentArg = currentArg + 1
				local arg = select(currentArg, ...)

				if formatSpecifier == "" then
					-- This should use the equivalent of Rust's 'Display', ie
					-- tostring and the __tostring metamethod.

					buffer:writeRaw(tostring(arg))
				elseif formatSpecifier == ":?" then
					-- This should use the equivalent of Rust's 'Debug',
					-- invented for this library as __fmtDebug.

					debugImpl(buffer, arg, false)
				elseif formatSpecifier == ":#?" then
					-- This should use the equivlant of Rust's 'Debug' with the
					-- 'alternate' (ie expanded) flag set.

					debugImpl(buffer, arg, true)
				else
					error("unsupported format specifier " .. formatSpecifier, 2)
				end

				i = closeBrace + 1
			end
		end
	end
end

local function debugOutputBuffer()
	local buffer = {}
	local startOfLine = true
	local indentLevel = 0
	local indentation = ""

	function buffer:writeLine(template, ...)
		writeFmt(self, template, ...)
		self:nextLine()
	end

	function buffer:writeLineRaw(value)
		self:writeRaw(value)
		self:nextLine()
	end

	function buffer:write(template, ...)
		return writeFmt(self, template, ...)
	end

	function buffer:writeRaw(value)
		if #value > 0 then
			if startOfLine and #indentation > 0 then
				startOfLine = false
				table.insert(self, indentation)
			end

			table.insert(self, value)
			startOfLine = false
		end
	end

	function buffer:nextLine()
		table.insert(self, "\n")
		startOfLine = true
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

local function fmt(template, ...)
	local buffer = debugOutputBuffer()
	writeFmt(buffer, template, ...)
	return buffer:finish()
end

--[[
	Wrap the given object in a type that implements the given function as its
	Debug implementation, and forwards __tostring to the type's underlying
	tostring implementation.
]]
local function debugify(object, fmtFunc)
	return setmetatable({}, {
		__fmtDebug = function(_, ...)
			return fmtFunc(object, ...)
		end,
		__tostring = function()
			return tostring(object)
		end,
	})
end

return {
	debugOutputBuffer = debugOutputBuffer,
	fmt = fmt,
	debugify = debugify,
}