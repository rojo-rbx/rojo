local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)

local function fromMotor(motor)
	local motorBinding, setMotorBinding = Roact.createBinding(motor:getValue())
	motor:onStep(setMotorBinding)
	return motorBinding
end

local function mapLerpColor(binding, color1, color2)
	return binding:map(function(value)
		return color1:Lerp(color2, value)
	end)
end

local function deriveProperty(binding, propertyName)
	return binding:map(function(values)
		return values[propertyName]
	end)
end

local function blendAlpha(alphaValues)
	local alpha

	for _, value in pairs(alphaValues) do
		alpha = alpha and alpha + (1 - alpha) * value or value
	end

	return alpha
end

return {
	fromMotor = fromMotor,
	mapLerpColor = mapLerpColor,
	deriveProperty = deriveProperty,
	blendAlpha = blendAlpha,
}