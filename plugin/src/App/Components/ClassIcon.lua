
local StudioService = game:GetService("StudioService")
local AssetService = game:GetService("AssetService")

type CachedImageInfo = {
	pixels: buffer,
	size: Vector2,
}

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local e = Roact.createElement

local EditableImage = require(Plugin.App.Components.EditableImage)

local imageCache = {} :: { [string]: CachedImageInfo }

local function getImageSizeAndPixels(image: string): (Vector2, buffer)
	local cached_image = imageCache[image]

	if not cached_image then
		local editableImage = AssetService:CreateEditableImageAsync(Content.fromUri(image))
		cached_image = {
			size = editableImage.Size,
			pixels = editableImage:ReadPixelsBuffer(Vector2.zero, editableImage.Size),
		}
		imageCache[image] = cached_image
	end

	return cached_image.size, cached_image.pixels
end

local function getRecoloredClassIcon(className, color)
	local iconProps = StudioService:GetClassIcon(className)

	if iconProps and color then
		--stylua: ignore
		local success, editableImageSize, editableImagePixels = pcall(@native function(iconProps: { [any]: any }, color: Color3): (Vector2, buffer)
			local size, pixels = getImageSizeAndPixels(iconProps.Image)
			local pixels_len = buffer.len(pixels)

			local minVal, maxVal = math.huge, -math.huge

			for i = 0, pixels_len, 4 do
				if buffer.readu8(pixels, i + 3) == 0 then
					continue
				end
				local pixelVal = math.max(
					buffer.readu8(pixels, i),
					buffer.readu8(pixels, i + 1),
					buffer.readu8(pixels, i + 2)
				)

				minVal = math.min(minVal, pixelVal)
				maxVal = math.max(maxVal, pixelVal)
			end

			local hue, sat, val = color:ToHSV()

			for i = 0, pixels_len, 4 do
				if buffer.readu8(pixels, i + 3) == 0 then
					continue
				end
				local g_index = i + 1
				local b_index = i + 2

				local pixelVal = math.max(
					buffer.readu8(pixels, i),
					buffer.readu8(pixels, g_index),
					buffer.readu8(pixels, b_index)
				)
				local newVal = val
				if minVal < maxVal then
					-- Remap minVal - maxVal to val*0.9 - val
					newVal = val * (0.9 + 0.1 * (pixelVal - minVal) / (maxVal - minVal))
				end

				local newPixelColor = Color3.fromHSV(hue, sat, newVal)
				buffer.writeu8(pixels, i, newPixelColor.R)
				buffer.writeu8(pixels, g_index, newPixelColor.G)
				buffer.writeu8(pixels, b_index, newPixelColor.B)
			end
			return size, pixels
		end, iconProps, color)
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
