--[[
	Fuzzy value-equality used to compare a decoded virtual property value against
	the live value read from a real instance. Shared by `diff` (to decide whether
	a property changed) and `hydrate` (to score candidate instances).
]]

local function fuzzyEq(a: number, b: number, epsilon: number): boolean
	return math.abs(a - b) < epsilon
end

local function trueEquals(a, b): boolean
	-- Exit early for simple equality values
	if a == b then
		return true
	end

	-- Treat nil and { Ref = "000...0" } as equal
	if
		(a == nil and type(b) == "table" and b.Ref == "00000000000000000000000000000000")
		or (b == nil and type(a) == "table" and a.Ref == "00000000000000000000000000000000")
	then
		return true
	end

	local typeA, typeB = typeof(a), typeof(b)

	-- For tables, try recursive deep equality
	if typeA == "table" and typeB == "table" then
		local checkedKeys = {}
		for key, value in a do
			checkedKeys[key] = true
			if not trueEquals(value, b[key]) then
				return false
			end
		end
		for key, value in b do
			if checkedKeys[key] then
				continue
			end
			if not trueEquals(value, a[key]) then
				return false
			end
		end
		return true

	-- For NaN, check if both values are not equal to themselves
	elseif a ~= a and b ~= b then
		return true

	-- For numbers, compare with epsilon of 0.0001 to avoid floating point inequality
	elseif typeA == "number" and typeB == "number" then
		return fuzzyEq(a, b, 0.0001)

	-- For EnumItem->number, compare the EnumItem's value
	elseif typeA == "number" and typeB == "EnumItem" then
		return a == b.Value
	elseif typeA == "EnumItem" and typeB == "number" then
		return a.Value == b

	-- For Color3s, compare to RGB ints to avoid floating point inequality
	elseif typeA == "Color3" and typeB == "Color3" then
		local aR, aG, aB = math.floor(a.R * 255), math.floor(a.G * 255), math.floor(a.B * 255)
		local bR, bG, bB = math.floor(b.R * 255), math.floor(b.G * 255), math.floor(b.B * 255)
		return aR == bR and aG == bG and aB == bB

	-- For CFrames, compare to components with epsilon of 0.0001 to avoid floating point inequality
	elseif typeA == "CFrame" and typeB == "CFrame" then
		local aComponents, bComponents = { a:GetComponents() }, { b:GetComponents() }
		for i, aComponent in aComponents do
			if not fuzzyEq(aComponent, bComponents[i], 0.0001) then
				return false
			end
		end
		return true

	-- For Vector3s, compare to components with epsilon of 0.0001 to avoid floating point inequality
	elseif typeA == "Vector3" and typeB == "Vector3" then
		local aComponents, bComponents = { a.X, a.Y, a.Z }, { b.X, b.Y, b.Z }
		for i, aComponent in aComponents do
			if not fuzzyEq(aComponent, bComponents[i], 0.0001) then
				return false
			end
		end
		return true

	-- For Vector2s, compare to components with epsilon of 0.0001 to avoid floating point inequality
	elseif typeA == "Vector2" and typeB == "Vector2" then
		local aComponents, bComponents = { a.X, a.Y }, { b.X, b.Y }
		for i, aComponent in aComponents do
			if not fuzzyEq(aComponent, bComponents[i], 0.0001) then
				return false
			end
		end
		return true
	end

	return false
end

return trueEquals
