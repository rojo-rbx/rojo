local RunService = game:GetService("RunService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)

local e = Roact.createElement
local TAU = math.pi * 2
local REDUCED_MOTION = false

-- Independent periods and phases keep the forms from falling into a visible loop.
local BLOBS = table.freeze({
	{
		colorA = Color3.fromHex("6B5CFF"),
		colorB = Color3.fromHex("D75CFF"),
		x = 0.08,
		y = 0.20,
		driftX = 0.20,
		driftY = 0.16,
		size = 260,
		period = 31,
		phase = 0.2,
	},
	{
		colorA = Color3.fromHex("42D9FF"),
		colorB = Color3.fromHex("4385FF"),
		x = 0.72,
		y = 0.12,
		driftX = 0.18,
		driftY = 0.20,
		size = 230,
		period = 37,
		phase = 1.7,
	},
	{
		colorA = Color3.fromHex("48E6AA"),
		colorB = Color3.fromHex("43CBE8"),
		x = 0.30,
		y = 0.78,
		driftX = 0.22,
		driftY = 0.12,
		size = 280,
		period = 29,
		phase = 3.1,
	},
	{
		colorA = Color3.fromHex("F0B653"),
		colorB = Color3.fromHex("EF648F"),
		x = 0.90,
		y = 0.72,
		driftX = 0.17,
		driftY = 0.18,
		size = 245,
		period = 23,
		phase = 4.4,
	},
	{
		colorA = Color3.fromHex("E85CB9"),
		colorB = Color3.fromHex("765CFF"),
		x = 0.52,
		y = 0.46,
		driftX = 0.14,
		driftY = 0.18,
		size = 210,
		period = 19,
		phase = 5.6,
	},
})

local function getTransform(blob, elapsed)
	local progress = elapsed / blob.period * TAU + blob.phase
	local x = blob.x + math.sin(progress) * blob.driftX
	local y = blob.y + math.cos(progress * 0.83) * blob.driftY
	local scale = 1 + math.sin(progress * 0.61 + blob.phase) * 0.06

	return UDim2.fromScale(x, y), UDim2.fromOffset(blob.size * scale, blob.size * scale)
end

local PrismBackground = Roact.Component:extend("PrismBackground")

PrismBackground.defaultProps = {
	active = true,
	reducedMotion = REDUCED_MOTION,
}

function PrismBackground:init()
	self.elapsed = 0
	self.blobRefs = {}
	for index in BLOBS do
		self.blobRefs[index] = Roact.createRef()
	end
end

function PrismBackground:updateBlobs(deltaTime)
	self.elapsed += deltaTime

	for index, blob in BLOBS do
		local object = self.blobRefs[index]:getValue()
		if object then
			object.Position, object.Size = getTransform(blob, self.elapsed)
		end
	end
end

function PrismBackground:startAnimation()
	if self.stepper or not self.props.active or self.props.reducedMotion then
		return
	end

	local stepSignal = self.props.stepSignal or RunService.RenderStepped
	self.stepper = stepSignal:Connect(function(deltaTime)
		self:updateBlobs(deltaTime)
	end)
end

function PrismBackground:stopAnimation()
	if self.stepper then
		self.stepper:Disconnect()
		self.stepper = nil
	end
end

function PrismBackground:didMount()
	self:startAnimation()
end

function PrismBackground:didUpdate(previousProps)
	if
		self.props.active ~= previousProps.active
		or self.props.reducedMotion ~= previousProps.reducedMotion
		or self.props.stepSignal ~= previousProps.stepSignal
	then
		self:stopAnimation()
		self:startAnimation()
	end
end

function PrismBackground:willUnmount()
	self:stopAnimation()
end

function PrismBackground:render()
	return Theme.with(function(theme)
		local blobs = {}
		for index, blob in BLOBS do
			local position, size = getTransform(blob, 0)
			blobs[`Blob{index}`] = e("ImageLabel", {
				[Roact.Ref] = self.blobRefs[index],
				Image = Assets.Images.Circles[500],
				ImageColor3 = Color3.new(1, 1, 1),
				ImageTransparency = 0.73,
				BackgroundTransparency = 1,
				Position = position,
				Size = size,
				AnchorPoint = Vector2.new(0.5, 0.5),
				ZIndex = 0,
			}, {
				Color = e("UIGradient", {
					Color = ColorSequence.new(blob.colorA, blob.colorB),
					Rotation = (index * 47) % 360,
					Transparency = NumberSequence.new({
						NumberSequenceKeypoint.new(0, 0.22),
						NumberSequenceKeypoint.new(0.5, 0),
						NumberSequenceKeypoint.new(1, 0.35),
					}),
				}),
				SoftCore = e("ImageLabel", {
					Image = Assets.Images.Circles[500],
					ImageColor3 = blob.colorB,
					ImageTransparency = 0.82,
					BackgroundTransparency = 1,
					Position = UDim2.fromScale(0.5, 0.5),
					Size = UDim2.fromScale(0.68, 0.68),
					AnchorPoint = Vector2.new(0.5, 0.5),
					ZIndex = 0,
				}),
			})
		end

		return e("Frame", {
			Size = UDim2.fromScale(1, 1),
			BackgroundColor3 = theme.Tokens.PanelBackground,
			BorderSizePixel = 0,
			ClipsDescendants = true,
			ZIndex = 0,
		}, {
			Blobs = Roact.createFragment(blobs),
			ReadabilityOverlay = e("Frame", {
				Size = UDim2.fromScale(1, 1),
				BackgroundColor3 = theme.Tokens.PanelBackground,
				BackgroundTransparency = 1 - theme.Tokens.OverlayDarkness,
				BorderSizePixel = 0,
				ZIndex = 1,
			}),
		})
	end)
end

PrismBackground._test = table.freeze({
	blobs = BLOBS,
	getTransform = getTransform,
})

return PrismBackground
