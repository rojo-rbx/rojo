local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Theme = require(Plugin.App.Theme)

local Toggle = require(Plugin.App.Components.Toggle)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local TextButton = require(Plugin.App.Components.TextButton)

local e = Roact.createElement

local PermissionPopup = Roact.Component:extend("PermissionPopup")

function PermissionPopup:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
	self.infoSize, self.setInfoSize = Roact.createBinding(Vector2.new(0, 0))

	local response = {}
	for _, api in self.props.apis do
		response[api] = if self.props.initialState[api] == nil then true else self.props.initialState[api]
	end

	self:setState({
		response = response,
	})
end

function PermissionPopup:render()
	return Theme.with(function(theme)
		theme = theme.Settings

		local apiToggles = {}
		for index, api in self.props.apis do
			apiToggles[api] = e(Toggle, {
				active = self.state.response[api],
				name = api,
				description = self.props.apiDescriptions[api],
				transparency = self.props.transparency,
				layoutOrder = index,
				onClick = function()
					self:setState(function(state)
						state.response[api] = not state.response[api]
						return state
					end)
				end,
			})
		end

		return e("Frame", {
			BackgroundTransparency = 1,
			Size = UDim2.new(1, 0, 1, 0),
		}, {
			Layout = e("UIListLayout", {
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 5),
				HorizontalAlignment = Enum.HorizontalAlignment.Right,
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
				PaddingTop = UDim.new(0, 15),
				PaddingBottom = UDim.new(0, 15),
			}),

			Info = e("TextLabel", {
				BackgroundTransparency = 1,
				Size = UDim2.new(1, 0, 0, 0),
				AutomaticSize = Enum.AutomaticSize.Y,
				Text = string.format("A third-party plugin, %s, is asking to use the following parts of the Rojo API. Please grant/deny access.", self.props.name or "[Unknown]"),
				Font = Enum.Font.GothamMedium,
				TextSize = 17,
				TextColor3 = theme.Setting.NameColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextWrapped = true,
				TextTransparency = self.props.transparency,
				LayoutOrder = 1,

				[Roact.Change.AbsoluteSize] = function(rbx)
					self.setInfoSize(rbx.AbsoluteSize)
				end,
			}),

			Submit = e(TextButton, {
				text = "Submit",
				style = "Solid",
				transparency = self.props.transparency,
				layoutOrder = 3,
				onClick = function()
					self.props.responseEvent:Fire(self.state.response)
				end,
			}),

			ScrollingFrame = e(ScrollingFrame, {
				size = self.infoSize:map(function(infoSize)
					return UDim2.new(1, 0, 1, -infoSize.Y-44)
				end),
				layoutOrder = 2,
				contentSize = self.contentSize,
				transparency = self.props.transparency,
			}, {
				APIToggles = Roact.createFragment(apiToggles),

				Padding = e("UIPadding", {
					PaddingRight = UDim.new(0, 15),
				}),

				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,

					[Roact.Change.AbsoluteContentSize] = function(object)
						self.setContentSize(object.AbsoluteContentSize)
					end,
				}),
			}),
		})
	end)
end

return PermissionPopup
