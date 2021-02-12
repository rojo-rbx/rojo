local base64 = require(script.Parent.base64)

local function identity(...)
	return ...
end

local function unpackDecoder(f)
	return function(value)
		return f(unpack(value))
	end
end

local function serializeFloat(value)
	-- TODO: Figure out a better way to serialize infinity and NaN, neither of
	-- which fit into JSON.
	if value == math.huge or value == -math.huge then
		return 999999999 * math.sign(value)
	end

	return value
end

-- enumsEncoder and enumsDecoder are used by datatypes that act as a bit-flag wrapper
-- for their underlying enum. This includes the 'Axes' and 'Faces' datatypes, which
-- act as bit-flags for 'Enum.Axis' and 'Enum.NormalId' respectively. By treating
-- the boolean value of each EnumItem as a bit, we can represent their value with
-- a single byte. 'Axes' uses 3 bits, while 'Faces' uses 6 bits.

local function enumsEncoder(enum)
	local items = enum:GetEnumItems()

	return function(flags)
		local mask = 0
		
		for _, item in ipairs(items) do
			if flags[item.Name] then
				mask += (2 ^ item.Value)
			end
		end
		
		return mask
	end
end

local function enumsDecoder(constructor, enum)
	local decode = unpackDecoder(constructor)
	local items = enum:GetEnumItems()

	return function(mask)
		local set = {}

		for _, item in ipairs(items) do
			local bit = (2 ^ item.Value)
			
			if bit32.btest(bit, mask) then
				table.insert(set, item)
			end
		end
		
		return decode(set)
	end
end

local encoders
encoders = {
	Bool = identity,
	Content = identity,
	Float32 = serializeFloat,
	Float64 = serializeFloat,
	Int32 = identity,
	Int64 = identity,
	String = identity,

	Axes = enumsEncoder(Enum.Axis),
	Faces = enumsEncoder(Enum.NormalId),

	BinaryString = base64.encode,
	SharedString = base64.encode,

	BrickColor = function(value)
		return value.Number
	end,

	CFrame = function(value)
		return {value:GetComponents()}
	end,
	Color3 = function(value)
		return {value.r, value.g, value.b}
	end,
	NumberRange = function(value)
		return {value.Min, value.Max}
	end,
	NumberSequence = function(value)
		local keypoints = {}

		for index, keypoint in ipairs(value.Keypoints) do
			keypoints[index] = {
				Time = keypoint.Time,
				Value = keypoint.Value,
				Envelope = keypoint.Envelope,
			}
		end

		return {
			Keypoints = keypoints,
		}
	end,
	ColorSequence = function(value)
		local keypoints = {}

		for index, keypoint in ipairs(value.Keypoints) do
			keypoints[index] = {
				Time = keypoint.Time,
				Color = encoders.Color3(keypoint.Value),
			}
		end

		return {
			Keypoints = keypoints,
		}
	end,
	Rect = function(value)
		return {
			Min = encoders.Vector2(value.Min),
			Max = encoders.Vector2(value.Max),
		}
	end,
	UDim = function(value)
		return {value.Scale, value.Offset}
	end,
	UDim2 = function(value)
		return {value.X.Scale, value.X.Offset, value.Y.Scale, value.Y.Offset}
	end,
	Vector2 = function(value)
		return {
			serializeFloat(value.X),
			serializeFloat(value.Y),
		}
	end,
	Vector2int16 = function(value)
		return {value.X, value.Y}
	end,
	Vector3 = function(value)
		return {
			serializeFloat(value.X),
			serializeFloat(value.Y),
			serializeFloat(value.Z),
		}
	end,
	Vector3int16 = function(value)
		return {value.X, value.Y, value.Z}
	end,

	PhysicalProperties = function(value)
		if value == nil then
			return nil
		else
			return {
				Density = value.Density,
				Friction = value.Friction,
				Elasticity = value.Elasticity,
				FrictionWeight = value.FrictionWeight,
				ElasticityWeight = value.ElasticityWeight,
			}
		end
	end,

	Ray = function(value)
		return {
			Origin = encoders.Vector3(value.Origin),
			Direction = encoders.Vector3(value.Direction),
		}
	end,

	Ref = function(value)
		return nil
	end,
}

local decoders
decoders = {
	Bool = identity,
	Content = identity,
	Enum = identity,
	Float32 = identity,
	Float64 = identity,
	Int32 = identity,
	Int64 = identity,
	String = identity,

	BinaryString = base64.decode,
	SharedString = base64.decode,

	Axes = enumsDecoder(Axes.new, Enum.Axis),
	Faces = enumsDecoder(Faces.new, Enum.NormalId),

	BrickColor = BrickColor.new,

	CFrame = unpackDecoder(CFrame.new),
	Color3 = unpackDecoder(Color3.new),
	Color3uint8 = unpackDecoder(Color3.fromRGB),
	NumberRange = unpackDecoder(NumberRange.new),
	UDim = unpackDecoder(UDim.new),
	UDim2 = unpackDecoder(UDim2.new),
	Vector2 = unpackDecoder(Vector2.new),
	Vector2int16 = unpackDecoder(Vector2int16.new),
	Vector3 = unpackDecoder(Vector3.new),
	Vector3int16 = unpackDecoder(Vector3int16.new),

	Rect = function(value)
		return Rect.new(
			decoders.Vector2(value.Min),
			decoders.Vector2(value.Max)
		)
	end,

	Ray = function(value)
		return Ray.new(
			decoders.Vector3(value.Origin),
			decoders.Vector3(value.Direction)
		)
	end,

	NumberSequence = function(value)
		local keypoints = {}

		for index, keypoint in ipairs(value.Keypoints) do
			keypoints[index] = NumberSequenceKeypoint.new(
				keypoint.Time,
				keypoint.Value,
				keypoint.Envelope
			)
		end

		return NumberSequence.new(keypoints)
	end,

	ColorSequence = function(value)
		local keypoints = {}

		for index, keypoint in ipairs(value.Keypoints) do
			keypoints[index] = ColorSequenceKeypoint.new(
				keypoint.Time,
				Color3.new(unpack(keypoint.Color))
			)
		end

		return ColorSequence.new(keypoints)
	end,

	PhysicalProperties = function(properties)
		if properties == nil then
			return nil
		else
			return PhysicalProperties.new(
				properties.Density,
				properties.Friction,
				properties.Elasticity,
				properties.FrictionWeight,
				properties.ElasticityWeight
			)
		end
	end,

	Ref = function()
		return nil
	end,
}

local EncodedValue = {}

function EncodedValue.decode(encodedValue)
	local decoder = decoders[encodedValue.Type]
	if decoder ~= nil then
		return true, decoder(encodedValue.Value)
	end

	return false, "Couldn't decode value " .. tostring(encodedValue.Type)
end

function EncodedValue.encode(rbxValue, propertyType)
	assert(propertyType ~= nil, "Property type descriptor is required")

	if propertyType.type == "Data" then
		local encoder = encoders[propertyType.name]

		if encoder == nil then
			return false, ("Missing encoder for property type %q"):format(propertyType.name)
		end

		if encoder ~= nil then
			return true, {
				Type = propertyType.name,
				Value = encoder(rbxValue),
			}
		end
	elseif propertyType.type == "Enum" then
		return true, {
			Type = "Enum",
			Value = rbxValue.Value,
		}
	end

	return false, ("Unknown property descriptor type %q"):format(tostring(propertyType.type))
end

return EncodedValue