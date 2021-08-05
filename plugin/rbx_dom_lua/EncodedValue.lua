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

local types
types = {
	Axes = {
		fromPod = function(pod)
			local axes = {}

			for index, axisName in ipairs(pod) do
				axes[index] = Enum.Axis[axisName]
			end

			return Axes.new(unpack(axes))
		end,

		toPod = function(roblox)
			local json = {}

			for _, axis in ipairs(ALL_AXES) do
				if roblox[axis] then
					table.insert(json, axis)
				end
			end

			return json
		end,
	},

	BinaryString = {
		fromPod = base64.decode,
		toPod = base64.encode,
	},

	Bool = {
		fromPod = identity,
		toPod = identity,
	},

	BrickColor = {
		fromPod = function(pod)
			return BrickColor.new(pod)
		end,

		toPod = function(roblox)
			return roblox.Number
		end,
	},

	CFrame = {
		fromPod = function(pod)
			local pos = pod.position
			local orient = pod.orientation

			return CFrame.new(
				pos[1], pos[2], pos[3],
				orient[1][1], orient[1][2], orient[1][3],
				orient[2][1], orient[2][2], orient[2][3],
				orient[3][1], orient[3][2], orient[3][3]
			)
		end,

		toPod = function(roblox)
			local x, y, z,
				r00, r01, r02,
				r10, r11, r12,
				r20, r21, r22 = roblox:GetComponents()

			return {
				position = {x, y, z},
				orientation = {
					{r00, r01, r02},
					{r10, r11, r12},
					{r20, r21, r22},
				},
			}
		end,
	},

	Color3 = {
		fromPod = unpackDecoder(Color3.new),

		toPod = function(roblox)
			return {roblox.r, roblox.g, roblox.b}
		end,
	},

	Color3uint8 = {
		fromPod = unpackDecoder(Color3.fromRGB),

		toPod = function(roblox)
			return {
				math.round(roblox.R * 255),
				math.round(roblox.G * 255),
				math.round(roblox.B * 255),
			}
		end,
	},

	ColorSequence = {
		fromPod = function(pod)
			local keypoints = {}

			for index, keypoint in ipairs(pod.keypoints) do
				keypoints[index] = ColorSequenceKeypoint.new(
					keypoint.time,
					types.Color3.fromPod(keypoint.color)
				)
			end

			return ColorSequence.new(keypoints)
		end,

		toPod = function(roblox)
			local keypoints = {}

			for index, keypoint in ipairs(roblox.Keypoints) do
				keypoints[index] = {
					time = keypoint.Time,
					color = types.Color3.toPod(keypoint.Value),
				}
			end

			return {
				keypoints = keypoints,
			}
		end,
	},

	Content = {
		fromPod = identity,
		toPod = identity,
	},

	Enum = {
		fromPod = identity,

		toPod = function(roblox)
			-- FIXME: More robust handling of enums
			if typeof(roblox) == "number" then
				return roblox
			else
				return roblox.Value
			end
		end,
	},

	Faces = {
		fromPod = function(pod)
			local faces = {}

			for index, faceName in ipairs(pod) do
				faces[index] = Enum.NormalId[faceName]
			end

			return Faces.new(unpack(faces))
		end,

		toPod = function(roblox)
			local pod = {}

			for _, face in ipairs(ALL_FACES) do
				if roblox[face] then
					table.insert(pod, face)
				end
			end

			return pod
		end,
	},

	Float32 = {
		fromPod = identity,
		toPod = serializeFloat,
	},

	Float64 = {
		fromPod = identity,
		toPod = serializeFloat,
	},

	Int32 = {
		fromPod = identity,
		toPod = identity,
	},

	Int64 = {
		fromPod = identity,
		toPod = identity,
	},

	NumberRange = {
		fromPod = unpackDecoder(NumberRange.new),

		toPod = function(roblox)
			return {roblox.Min, roblox.Max}
		end,
	},

	NumberSequence = {
		fromPod = function(pod)
			local keypoints = {}

			for index, keypoint in ipairs(pod.keypoints) do
				keypoints[index] = NumberSequenceKeypoint.new(
					keypoint.time,
					keypoint.value,
					keypoint.envelope
				)
			end

			return NumberSequence.new(keypoints)
		end,

		toPod = function(roblox)
			local keypoints = {}

			for index, keypoint in ipairs(roblox.Keypoints) do
				keypoints[index] = {
					time = keypoint.Time,
					value = keypoint.Value,
					envelope = keypoint.Envelope,
				}
			end

			return {
				keypoints = keypoints,
			}
		end,
	},

	PhysicalProperties = {
		fromPod = function(pod)
			if pod == "Default" then
				return nil
			else
				return PhysicalProperties.new(
					pod.density,
					pod.friction,
					pod.elasticity,
					pod.frictionWeight,
					pod.elasticityWeight
				)
			end
		end,

		toPod = function(roblox)
			if roblox == nil then
				return "Default"
			else
				return {
					density = roblox.Density,
					friction = roblox.Friction,
					elasticity = roblox.Elasticity,
					frictionWeight = roblox.FrictionWeight,
					elasticityWeight = roblox.ElasticityWeight,
				}
			end
		end,
	},

	Ray = {
		fromPod = function(pod)
			return Ray.new(
				types.Vector3.fromPod(pod.origin),
				types.Vector3.fromPod(pod.direction)
			)
		end,

		toPod = function(roblox)
			return {
				origin = types.Vector3.toPod(roblox.Origin),
				direction = types.Vector3.toPod(roblox.Direction),
			}
		end,
	},

	Rect = {
		fromPod = function(pod)
			return Rect.new(
				types.Vector2.fromPod(pod[1]),
				types.Vector2.fromPod(pod[2])
			)
		end,

		toPod = function(roblox)
			return {
				types.Vector2.toPod(roblox.Min),
				types.Vector2.toPod(roblox.Max),
			}
		end,
	},

	Ref = {
		fromPod = function(_pod)
			error("Ref cannot be decoded on its own")
		end,

		toPod = function(_roblox)
			error("Ref can not be encoded on its own")
		end,
	},

	Region3 = {
		fromPod = function(pod)
			error("Region3 is not implemented")
		end,

		toPod = function(roblox)
			error("Region3 is not implemented")
		end,
	},

	Region3int16 = {
		fromPod = function(pod)
			return Region3int16.new(
				types.Vector3int16.fromPod(pod[1]),
				types.Vector3int16.fromPod(pod[2])
			)
		end,

		toPod = function(roblox)
			return {
				types.Vector3int16.toPod(roblox.Min),
				types.Vector3int16.toPod(roblox.Max),
			}
		end,
	},

	SharedString = {
		fromPod = function(pod)
			error("SharedString is not supported")
		end,

		toPod = function(roblox)
			error("SharedString is not supported")
		end,
	},

	String = {
		fromPod = identity,
		toPod = identity,
	},

	UDim = {
		fromPod = unpackDecoder(UDim.new),

		toPod = function(roblox)
			return {roblox.Scale, roblox.Offset}
		end,
	},

	UDim2 = {
		fromPod = function(pod)
			return UDim2.new(
				types.UDim.fromPod(pod[1]),
				types.UDim.fromPod(pod[2])
			)
		end,

		toPod = function(roblox)
			return {
				types.UDim.toPod(roblox.X),
				types.UDim.toPod(roblox.Y),
			}
		end,
	},

	Vector2 = {
		fromPod = unpackDecoder(Vector2.new),

		toPod = function(roblox)
			return {
				serializeFloat(roblox.X),
				serializeFloat(roblox.Y),
			}
		end,
	},

	Vector2int16 = {
		fromPod = unpackDecoder(Vector2int16.new),

		toPod = function(roblox)
			return {roblox.X, roblox.Y}
		end,
	},

	Vector3 = {
		fromPod = unpackDecoder(Vector3.new),

		toPod = function(roblox)
			return {
				serializeFloat(roblox.X),
				serializeFloat(roblox.Y),
				serializeFloat(roblox.Z),
			}
		end,
	},

	Vector3int16 = {
		fromPod = unpackDecoder(Vector3int16.new),

		toPod = function(roblox)
			return {roblox.X, roblox.Y, roblox.Z}
		end,
	},
}

local EncodedValue = {}

function EncodedValue.decode(encodedValue)
	local ty, value = next(encodedValue)

	local typeImpl = types[ty]
	if typeImpl == nil then
		return false, "Couldn't decode value " .. tostring(ty)
	end

	return true, typeImpl.fromPod(value)
end

function EncodedValue.encode(rbxValue, propertyType)
	assert(propertyType ~= nil, "Property type descriptor is required")

	local typeImpl = types[propertyType]
	if typeImpl == nil then
		return false, ("Missing encoder for property type %q"):format(propertyType)
	end

	return true, {
		[propertyType] = typeImpl.toPod(rbxValue),
	}
end

return EncodedValue
