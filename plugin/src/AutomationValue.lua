local MAX_DEPTH = 32
local MAX_ELEMENTS = 10_000
local MAX_STRING_BYTES = 64 * 1024

local AutomationValue = {}

local function finite(value)
	return value == value and value ~= math.huge and value ~= -math.huge
end

function AutomationValue.encode(value, references, diagnosticPath: string?)
	local seen = {}
	local elements = 0

	local function encode(current, depth, path)
		elements += 1
		if elements > MAX_ELEMENTS then
			return nil, "Automation value exceeds the element limit"
		end
		if depth > MAX_DEPTH then
			return nil, "Automation value exceeds the depth limit"
		end

		local valueType = typeof(current)
		if valueType == "nil" then
			return { kind = "nil" }
		elseif valueType == "boolean" then
			return { kind = "boolean", value = current }
		elseif valueType == "number" then
			if not finite(current) then
				return nil, "Automation value contains a non-finite number at " .. path
			end
			return { kind = "number", value = current }
		elseif valueType == "string" then
			if #current > MAX_STRING_BYTES then
				return nil, "Automation string exceeds the 64 KiB limit at " .. path
			end
			return { kind = "string", value = current }
		elseif valueType == "Vector2" then
			return { kind = "vector2", x = current.X, y = current.Y }
		elseif valueType == "Vector3" then
			return { kind = "vector3", x = current.X, y = current.Y, z = current.Z }
		elseif valueType == "Color3" then
			return { kind = "color3", r = current.R, g = current.G, b = current.B }
		elseif valueType == "CFrame" then
			return { kind = "cFrame", components = { current:GetComponents() } }
		elseif valueType == "UDim" then
			return { kind = "uDim", scale = current.Scale, offset = current.Offset }
		elseif valueType == "UDim2" then
			return {
				kind = "uDim2",
				x = { scale = current.X.Scale, offset = current.X.Offset },
				y = { scale = current.Y.Scale, offset = current.Y.Offset },
			}
		elseif valueType == "Rect" then
			return {
				kind = "rect",
				min = { x = current.Min.X, y = current.Min.Y },
				max = { x = current.Max.X, y = current.Max.Y },
			}
		elseif valueType == "EnumItem" then
			return { kind = "enumItem", enumType = tostring(current.EnumType):gsub("^Enum%.", ""), name = current.Name }
		elseif valueType == "BrickColor" then
			return { kind = "brickColor", number = current.Number, name = current.Name }
		elseif valueType == "NumberRange" then
			return { kind = "numberRange", min = current.Min, max = current.Max }
		elseif valueType == "Instance" then
			if references == nil then
				return nil, "Instance value cannot be encoded without a reference registry"
			end
			local reference, referenceError = references:reference(current, diagnosticPath or path)
			if reference == nil then
				return nil, referenceError
			end
			return { kind = "instanceReference", value = reference }
		elseif valueType ~= "table" then
			return nil, string.format("Unsupported automation value type '%s' at %s", valueType, path)
		end

		if seen[current] then
			return nil, "Automation value contains a cycle at " .. path
		end
		seen[current] = true
		local numericCount = 0
		local maximumIndex = 0
		local stringKeys = {}
		for key in current do
			if type(key) == "number" and key >= 1 and key % 1 == 0 then
				numericCount += 1
				maximumIndex = math.max(maximumIndex, key)
			elseif type(key) == "string" then
				table.insert(stringKeys, key)
			else
				seen[current] = nil
				return nil, "Automation tables require positive integer or string keys"
			end
		end
		if numericCount > 0 and #stringKeys > 0 then
			seen[current] = nil
			return nil, "Automation tables cannot mix array and dictionary keys"
		end

		local encoded
		if numericCount > 0 then
			if maximumIndex ~= numericCount then
				seen[current] = nil
				return nil, "Automation arrays must be dense"
			end
			local values = table.create(numericCount)
			for index = 1, numericCount do
				local child, childError = encode(current[index], depth + 1, string.format("%s[%d]", path, index))
				if child == nil then
					seen[current] = nil
					return nil, childError
				end
				values[index] = child
			end
			encoded = { kind = "array", value = values }
		else
			table.sort(stringKeys)
			local values = table.create(#stringKeys)
			for index, key in stringKeys do
				local child, childError = encode(current[key], depth + 1, path .. "." .. key)
				if child == nil then
					seen[current] = nil
					return nil, childError
				end
				values[index] = { key = key, value = child }
			end
			encoded = { kind = "map", value = values }
		end
		seen[current] = nil
		return encoded
	end

	return encode(value, 1, "value")
end

function AutomationValue.diagnostic(errorValue)
	return { kind = "diagnostic", error = tostring(errorValue) }
end

return AutomationValue
