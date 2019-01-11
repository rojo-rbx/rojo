local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local Config = require(script.Parent.Parent.Config)

local FitList = require(script.Parent.FitList)
local FitText = require(script.Parent.FitText)

local e = Roact.createElement

local FORM_TEXT_SIZE = 20

local ConnectPanel = Roact.Component:extend("ConnectPanel")

function ConnectPanel:init()
	self.labelSizes = {}
	self.labelSize, self.setLabelSize = Roact.createBinding(Vector2.new())

	self:setState({
		address = Config.defaultHost,
		port = Config.defaultPort,
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
			BackgroundColor3 = Color3.fromRGB(32, 32, 32),
			BorderColor3 = Color3.fromRGB(64, 64, 64),
			Position = UDim2.new(0.5, 0, 0, 0),
			AnchorPoint = Vector2.new(0.5, 0),
		},
	}, {
		Title = e("TextLabel", {
			LayoutOrder = 1,
			Font = Enum.Font.SourceSans,
			TextSize = 22,
			Text = "Start New Rojo Session",
			Size = UDim2.new(1, 0, 0, 28),
			BackgroundTransparency = 1,
			TextColor3 = Color3.new(1, 1, 1),
		}, {
			BottomBorder = e("Frame", {
				BorderSizePixel = 0,
				BackgroundColor3 = Color3.fromRGB(48, 48, 48),
				Size = UDim2.new(1, 0, 0, 1),
				Position = UDim2.new(0, 0, 1, -1),
			}),
		}),

		Body = e(FitList, {
			containerProps = {
				BackgroundTransparency = 1,
				LayoutOrder = 2,
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
					MinSize = Vector2.new(0, 24),
					Kind = "TextLabel",
					LayoutOrder = 1,
					BackgroundTransparency = 1,
					TextXAlignment = Enum.TextXAlignment.Left,
					Font = Enum.Font.SourceSans,
					TextSize = FORM_TEXT_SIZE,
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

				InputOuter = e("Frame", {
					LayoutOrder = 2,
					Size = UDim2.new(0, 300, 0, 24),
					BackgroundColor3 = Color3.fromRGB(32, 32, 32),
					BorderColor3 = Color3.fromRGB(64, 64, 64),
				}, {
					InputInner = e("TextBox", {
						BackgroundTransparency = 1,
						Size = UDim2.new(1, -8, 1, -8),
						Position = UDim2.new(0.5, 0, 0.5, 0),
						AnchorPoint = Vector2.new(0.5, 0.5),
						Font = Enum.Font.SourceSans,
						ClearTextOnFocus = false,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextSize = FORM_TEXT_SIZE,
						Text = self.state.address,
						TextColor3 = Color3.fromRGB(245, 245, 245),

						[Roact.Change.Text] = function(rbx)
							self:setState({
								address = rbx.Text,
							})
						end,
					}),
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
					MinSize = Vector2.new(0, 24),
					Kind = "TextLabel",
					LayoutOrder = 1,
					BackgroundTransparency = 1,
					TextXAlignment = Enum.TextXAlignment.Left,
					Font = Enum.Font.SourceSans,
					TextSize = FORM_TEXT_SIZE,
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

				InputOuter = e("Frame", {
					LayoutOrder = 2,
					Size = UDim2.new(0, 300, 0, 24),
					BackgroundColor3 = Color3.fromRGB(32, 32, 32),
					BorderColor3 = Color3.fromRGB(64, 64, 64),
				}, {
					InputInner = e("TextBox", {
						BackgroundTransparency = 1,
						Size = UDim2.new(1, -8, 1, -8),
						Position = UDim2.new(0.5, 0, 0.5, 0),
						AnchorPoint = Vector2.new(0.5, 0.5),
						Font = Enum.Font.SourceSans,
						ClearTextOnFocus = false,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextSize = FORM_TEXT_SIZE,
						Text = self.state.port,
						TextColor3 = Color3.fromRGB(245, 245, 245),

						[Roact.Change.Text] = function(rbx)
							self:setState({
								port = rbx.Text,
							})
						end,
					}),
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
					BackgroundColor3 = Color3.fromRGB(32, 32, 32),
					BorderColor3 = Color3.fromRGB(64, 64, 64),
					TextColor3 = Color3.fromRGB(245, 245, 245),
					Text = "Start",
					Font = Enum.Font.SourceSans,
					TextSize = FORM_TEXT_SIZE,
					Padding = Vector2.new(12, 3),

					[Roact.Event.Activated] = function()
						if startSession ~= nil then
							startSession(self.state.address, self.state.port)
						end
					end,
				}),

				e(FitText, {
					Kind = "TextButton",
					LayoutOrder = 2,
					BackgroundColor3 = Color3.fromRGB(32, 32, 32),
					BorderColor3 = Color3.fromRGB(64, 64, 64),
					TextColor3 = Color3.fromRGB(245, 245, 245),
					Text = "Cancel",
					Font = Enum.Font.SourceSans,
					TextSize = FORM_TEXT_SIZE,
					Padding = Vector2.new(12, 3),

					[Roact.Event.Activated] = function()
						if cancel ~= nil then
							cancel()
						end
					end,
				}),
			})
		})
	})
end

return ConnectPanel