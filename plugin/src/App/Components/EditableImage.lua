local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local e = Roact.createElement

local EditableImage = Roact.PureComponent:extend("EditableImage")

function EditableImage:init()
	self.ref = Roact.createRef()
end

function EditableImage:writePixels()
	local image = self.ref.current :: EditableImage
	local props = self.props

	if not image then
		return
	end
	if not props.pixels then
		return
	end

	image:WritePixelsBuffer(Vector2.zero, props.size, props.pixels)
end

function EditableImage:render()
	return e("EditableImage", {
		Size = self.props.size,
		[Roact.Ref] = self.ref,
	})
end

function EditableImage:didMount()
	self:writePixels()
end

function EditableImage:didUpdate()
	self:writePixels()
end

return EditableImage
