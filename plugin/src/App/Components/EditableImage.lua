local Rojo = script:FindFirstAncestor("Rojo")
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local e = Roact.createElement

local EditableImage = Roact.PureComponent:extend("EditableImage")

function EditableImage:init()
	self.ref = Roact.createRef()
end

function EditableImage:writePixels()
	local image = self.ref.current
	if not image then
		return
	end
	if not self.props.pixels then
		return
	end

	image:WritePixels(Vector2.zero, self.props.size, self.props.pixels)
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
