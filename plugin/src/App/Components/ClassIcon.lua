local StudioService = game:GetService("StudioService")
local AssetService = game:GetService("AssetService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local e = Roact.createElement

local EditableImage = require(Plugin.App.Components.EditableImage)

local imageCache = {}
local function getImageSizeAndPixels(image)
	if not imageCache[image] then
		local editableImage = AssetService:CreateEditableImageAsync(image)
		imageCache[image] = {
			Size = editableImage.Size,
			Pixels = editableImage:ReadPixels(Vector2.zero, editableImage.Size),
		}
	end

	return imageCache[image].Size, table.clone(imageCache[image].Pixels)
end

local function getRecoloredClassIcon(className, color)
	local iconProps = StudioService:GetClassIcon(className)

	if iconProps and color then
		local success, editableImageSize, editableImagePixels = pcall(function()
			local size, pixels = getImageSizeAndPixels(iconProps.Image)

			local minVal, maxVal = math.huge, -math.huge
			for i = 1, #pixels, 4 do
				if pixels[i + 3] == 0 then
					continue
				end
				local pixelVal = math.max(pixels[i], pixels[i + 1], pixels[i + 2])

				minVal = math.min(minVal, pixelVal)
				maxVal = math.max(maxVal, pixelVal)
			end

			local hue, sat, val = color:ToHSV()
			for i = 1, #pixels, 4 do
				if pixels[i + 3] == 0 then
					continue
				end

				local pixelVal = math.max(pixels[i], pixels[i + 1], pixels[i + 2])
				local newVal = val
				if minVal < maxVal then
					-- Remap minVal - maxVal to val*0.9 - val
					newVal = val * (0.9 + 0.1 * (pixelVal - minVal) / (maxVal - minVal))
				end

				local newPixelColor = Color3.fromHSV(hue, sat, newVal)
				pixels[i], pixels[i + 1], pixels[i + 2] = newPixelColor.R, newPixelColor.G, newPixelColor.B
			end
			return size, pixels
		end)
		if success then
			iconProps.EditableImagePixels = editableImagePixels
			iconProps.EditableImageSize = editableImageSize
		end
	end

	return iconProps
end

local ClassIcon = Roact.PureComponent:extend("ClassIcon")

function ClassIcon:init()
	self.state = {
		iconProps = nil,
	}
end

function ClassIcon:updateIcon()
	local props = self.props
	local iconProps = getRecoloredClassIcon(props.className, props.color)
	self:setState({
		iconProps = iconProps,
	})
end

function ClassIcon:didMount()
	self:updateIcon()
end

function ClassIcon:didUpdate(lastProps)
	if lastProps.className ~= self.props.className or lastProps.color ~= self.props.color then
		self:updateIcon()
	end
end

function ClassIcon:render()
	local iconProps = self.state.iconProps
	if not iconProps then
		return nil
	end

	return e(
		"ImageLabel",
		{
			Size = self.props.size,
			Position = self.props.position,
			LayoutOrder = self.props.layoutOrder,
			AnchorPoint = self.props.anchorPoint,
			ImageTransparency = self.props.transparency,
			Image = iconProps.Image,
			ImageRectOffset = iconProps.ImageRectOffset,
			ImageRectSize = iconProps.ImageRectSize,
			BackgroundTransparency = 1,
		},
		if iconProps.EditableImagePixels
			then e(EditableImage, {
				size = iconProps.EditableImageSize,
				pixels = iconProps.EditableImagePixels,
			})
			else nil
	)
end

return ClassIcon
