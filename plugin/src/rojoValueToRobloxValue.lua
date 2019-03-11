local primitiveTypes = {
	Bool = true,
	Enum = true,
	Float32 = true,
	Float64 = true,
	Int32 = true,
	Int64 = true,
	String = true,
}

local directConstructors = {
	CFrame = CFrame.new,
	Color3 = Color3.new,
	Color3uint8 = Color3.fromRGB,
	Rect = Rect.new,
	UDim = UDim.new,
	UDim2 = UDim2.new,
	Vector2 = Vector2.new,
	Vector2int16 = Vector2int16.new,
	Vector3 = Vector3.new,
	Vector3int16 = Vector3int16.new,
}

local function rojoValueToRobloxValue(value)
	if primitiveTypes[value.Type] then
		return value.Value
	end

	local constructor = directConstructors[value.Type]
	if constructor ~= nil then
		return constructor(unpack(value.Value))
	end

	local errorMessage = ("The Rojo plugin doesn't know how to handle values of type %q yet!"):format(tostring(value.Type))
	error(errorMessage)
end

return rojoValueToRobloxValue