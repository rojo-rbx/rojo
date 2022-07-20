local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local e = Roact.createElement

local function SlicedImage(props)
	local slice = props.slice

	return e("ImageLabel", {
		Image = slice.Image,
		ImageColor3 = props.color,
		ImageTransparency = props.transparency,

		ScaleType = Enum.ScaleType.Slice,
		SliceCenter = slice.Center,
		SliceScale = slice.Scale,

		Size = props.size,
		Position = props.position,
		AnchorPoint = props.anchorPoint,

		ZIndex = props.zIndex,
		LayoutOrder = props.layoutOrder,
		BackgroundTransparency = 1,
	}, props[Roact.Children])
end

return SlicedImage
