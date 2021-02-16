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

local ALL_AXES = {"X", "Y", "Z"}
local ALL_FACES = {"Right", "Top", "Back", "Left", "Bottom", "Front"}

local encoders
encoders = {
	Bool = identity,
	Content = identity,
	Float32 = serializeFloat,
	Float64 = serializeFloat,
	Int32 = identity,
	Int64 = identity,
	String = identity,

	BinaryString = base64.encode,
	SharedString = base64.encode,

	Axes = function(value)
		local output = {}

		for _, axis in ipairs(ALL_AXES) do
			if value[axis] then
				table.insert(output, axis)
			end
		end

		return output
	end,

	Faces = function(value)
		local output = {}

		for _, face in ipairs(ALL_FACES) do
			if value[face] then
				table.insert(output, face)
			end
		end

		return output
	end,

	Enum = function(value)
		if typeof(value) == "number" then
			return value
		else
			return value.Value
		end
	end,

	BrickColor = function(value)
		return value.Number
	end,

	CFrame = function(value)
		local x, y, z,
			r00, r01, r02,
			r10, r11, r12,
			r20, r21, r22 = value:GetComponents()

		return {
			Position = {x, y, z},
			Orientation = {
				{r00, r10, r20},
				{r01, r11, r21},
				{r02, r12, r22},
			},
		}
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
			encoders.Vector2(value.Min),
			encoders.Vector2(value.Max),
		}
	end,
	UDim = function(value)
		return {value.Scale, value.Offset}
	end,
	UDim2 = function(value)
		return {
			encoders.UDim(value.X),
			encoders.UDim(value.Y),
		}
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

	Region3int16 = function(value)
		return {
			encoders.Vector3int16(value.Min),
			encoders.Vector3int16(value.Max),
		}
	end,

	Color3uint8 = function(value)
		return {
			math.round(value.R * 255),
			math.round(value.G * 255),
			math.round(value.B * 255),
		}
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

	BrickColor = BrickColor.new,

	Color3 = unpackDecoder(Color3.new),
	Color3uint8 = unpackDecoder(Color3.fromRGB),
	NumberRange = unpackDecoder(NumberRange.new),
	UDim = unpackDecoder(UDim.new),
	Vector2 = unpackDecoder(Vector2.new),
	Vector2int16 = unpackDecoder(Vector2int16.new),
	Vector3 = unpackDecoder(Vector3.new),
	Vector3int16 = unpackDecoder(Vector3int16.new),

	UDim2 = function(value)
		return UDim2.new(
			value[1][1],
			value[1][2],
			value[2][1],
			value[2][2]
		)
	end,

	Axes = function(value)
		local axes = {}
		for index, axisName in ipairs(value) do
			axes[index] = Enum.Axis[axisName]
		end

		return Axes.new(unpack(axes))
	end,

	Faces = function(value)
		local normalIds = {}
		for index, faceName in ipairs(value) do
			normalIds[index] = Enum.NormalId[faceName]
		end

		return Faces.new(unpack(normalIds))
	end,

	CFrame = function(value)
		return CFrame.fromMatrix(
			decoders.Vector3(value.Position),
			decoders.Vector3(value.Orientation[1]),
			decoders.Vector3(value.Orientation[2]),
			decoders.Vector3(value.Orientation[3])
		)
	end,

	Rect = function(value)
		return Rect.new(
			decoders.Vector2(value[1]),
			decoders.Vector2(value[2])
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

	Region3int16 = function(value)
		return Region3int16.new(
			decoders.Vector3int16(value[1]),
			decoders.Vector3int16(value[2])
		)
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

	local encoder = encoders[propertyType]

	if encoder == nil then
		return false, ("Missing encoder for property type %q"):format(propertyType)
	end

	return true, {
		Type = propertyType,
		Value = encoder(rbxValue),
	}
end

return EncodedValue
