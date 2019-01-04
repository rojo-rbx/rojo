local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local FitList = require(script.Parent.FitList)
local FitText = require(script.Parent.FitText)

local e = Roact.createElement

local ConnectPanel = Roact.Component:extend("ConnectPanel")

function ConnectPanel:init()
	self.labelSizes = {}
	self.labelSize, self.setLabelSize = Roact.createBinding(Vector2.new())

	self:setState({
		address = "localhost",
		port = "34872",
	})
end

function ConnectPanel:updateLabelSize(name, size)
	self.labelSizes[name] = size

	local x = 0
	local y = 0

	for _, size in pairs(self.labelSizes) do
		x = math.max(x, size.X)
		y = math.max(y, size.Y)
	end

	self.setLabelSize(Vector2.new(x, y))
end

function ConnectPanel:render()
	local startSession = self.props.startSession
	local cancel = self.props.cancel

	return e(FitList, {
		containerProps = {
			BackgroundColor3 = Color3.fromRGB(8, 8, 8),
			BorderColor3 = Color3.fromRGB(64, 64, 64),
			Position = UDim2.new(0.5, 0, 0, 0),
			AnchorPoint = Vector2.new(0.5, 0),
		},
		layoutProps = {
			Padding = UDim.new(0, 8),
		},
		paddingProps = {
			PaddingTop = UDim.new(0, 8),
			PaddingBottom = UDim.new(0, 8),
			PaddingLeft = UDim.new(0, 8),
			PaddingRight = UDim.new(0, 8),
		},
	}, {
		Address = e(FitList, {
			containerProps = {
				LayoutOrder = 1,
				BackgroundTransparency = 1,
			},
			layoutProps = {
				FillDirection = Enum.FillDirection.Horizontal,
				Padding = UDim.new(0, 8),
			},
		}, {
			Label = e(FitText, {
				Kind = "TextLabel",
				LayoutOrder = 1,
				BackgroundTransparency = 1,
				TextXAlignment = Enum.TextXAlignment.Left,
				Font = Enum.Font.SourceSans,
				TextSize = 16,
				Text = "Address",
				TextColor3 = Color3.fromRGB(245, 245, 245),

				[Roact.Change.AbsoluteSize] = function(rbx)
					self:updateLabelSize("address", rbx.AbsoluteSize)
				end,
			}, {
				Sizing = e("UISizeConstraint", {
					MinSize = self.labelSize,
				}),
			}),

			Input = e("TextBox", {
				LayoutOrder = 2,
				Size = UDim2.new(0, 300, 0, 20),
				Font = Enum.Font.SourceSans,
				ClearTextOnFocus = false,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextSize = 16,
				Text = self.state.address,
				TextColor3 = Color3.fromRGB(245, 245, 245),
				BackgroundColor3 = Color3.fromRGB(8, 8, 8),
				BorderColor3 = Color3.fromRGB(64, 64, 64),

				[Roact.Change.Text] = function(rbx)
					self:setState({
						address = rbx.Text,
					})
				end,
			}),
		}),

		Port = e(FitList, {
			containerProps = {
				LayoutOrder = 2,
				BackgroundTransparency = 1,
			},
			layoutProps = {
				FillDirection = Enum.FillDirection.Horizontal,
				Padding = UDim.new(0, 8),
			},
		}, {
			Label = e(FitText, {
				Kind = "TextLabel",
				LayoutOrder = 1,
				BackgroundTransparency = 1,
				TextXAlignment = Enum.TextXAlignment.Left,
				Font = Enum.Font.SourceSans,
				TextSize = 16,
				Text = "Port",
				TextColor3 = Color3.fromRGB(245, 245, 245),

				[Roact.Change.AbsoluteSize] = function(rbx)
					self:updateLabelSize("port", rbx.AbsoluteSize)
				end,
			}, {
				Sizing = e("UISizeConstraint", {
					MinSize = self.labelSize,
				}),
			}),

			Input = e("TextBox", {
				LayoutOrder = 2,
				Size = UDim2.new(0, 300, 0, 20),
				Font = Enum.Font.SourceSans,
				ClearTextOnFocus = false,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextSize = 16,
				Text = self.state.port,
				TextColor3 = Color3.fromRGB(245, 245, 245),
				BackgroundColor3 = Color3.fromRGB(8, 8, 8),
				BorderColor3 = Color3.fromRGB(64, 64, 64),

				[Roact.Change.Text] = function(rbx)
					self:setState({
						port = rbx.Text,
					})
				end,
			}),
		}),

		Buttons = e(FitList, {
			containerProps = {
				LayoutOrder = 3,
				BackgroundTransparency = 1,
			},
			layoutProps = {
				FillDirection = Enum.FillDirection.Horizontal,
				Padding = UDim.new(0, 8),
			},
		}, {
			e(FitText, {
				Kind = "TextButton",
				LayoutOrder = 1,
				BackgroundColor3 = Color3.fromRGB(16, 16, 16),
				BorderColor3 = Color3.fromRGB(64, 64, 64),
				TextColor3 = Color3.fromRGB(245, 245, 245),
				Text = "Start",

				[Roact.Event.Activated] = function()
					if startSession ~= nil then
						startSession(self.state.address, self.state.port)
					end
				end,
			}),

			e(FitText, {
				Kind = "TextButton",
				LayoutOrder = 2,
				BackgroundColor3 = Color3.fromRGB(16, 16, 16),
				BorderColor3 = Color3.fromRGB(64, 64, 64),
				TextColor3 = Color3.fromRGB(245, 245, 245),
				Text = "Cancel",

				[Roact.Event.Activated] = function()
					if cancel ~= nil then
						cancel()
					end
				end,
			}),
		})
	})
end

return ConnectPanel