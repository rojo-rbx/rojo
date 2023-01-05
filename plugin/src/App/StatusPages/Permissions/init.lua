local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)
local Log = require(Packages.Log)

local Assets = require(Plugin.Assets)
local Theme = require(Plugin.App.Theme)

local IconButton = require(Plugin.App.Components.IconButton)
local ScrollingFrame = require(Plugin.App.Components.ScrollingFrame)
local Tooltip = require(Plugin.App.Components.Tooltip)
local Listing = require(script.Listing)

local e = Roact.createElement

local function Navbar(props)
	return Theme.with(function(theme)
		theme = theme.Settings.Navbar

		return e("Frame", {
			Size = UDim2.new(1, 0, 0, 46),
			LayoutOrder = props.layoutOrder,
			BackgroundTransparency = 1,
		}, {
			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
			}),

			Back = e(IconButton, {
				icon = Assets.Images.Icons.Back,
				iconSize = 24,
				color = theme.BackButtonColor,
				transparency = props.transparency,

				position = UDim2.new(0, 0, 0.5, 0),
				anchorPoint = Vector2.new(0, 0.5),

				onClick = props.onBack,
			}, {
				Tip = e(Tooltip.Trigger, {
					text = "Back"
				}),
			}),

			Text = e("TextLabel", {
				Text = "Permissions",
				Font = Enum.Font.Gotham,
				TextSize = 18,
				TextColor3 = theme.TextColor,
				TextTransparency = props.transparency,

				Size = UDim2.new(1, 0, 1, 0),

				BackgroundTransparency = 1,
			})
		})
	end)
end

local PermissionsPage = Roact.Component:extend("PermissionsPage")

function PermissionsPage:init()
	self.contentSize, self.setContentSize = Roact.createBinding(Vector2.new(0, 0))
end

function PermissionsPage:render()
	return Theme.with(function(theme)
		theme = theme.Settings

		local sources = {}
		if next(self.props.headlessAPI._permissions) == nil then
			sources.noSources = e("TextLabel", {
				Text = "No third-party plugins have been granted permissions.",
				Font = Enum.Font.Gotham,
				TextSize = 18,
				TextColor3 = theme.Setting.DescriptionColor,
				TextTransparency = self.props.transparency,
				TextWrapped = true,

				Size = UDim2.new(1, 0, 0, 48),
				LayoutOrder = 0,

				BackgroundTransparency = 1,
			})
		else
			for source in self.props.headlessAPI._permissions do
				local meta = self.props.headlessAPI:_getMetaFromSource(source)
				sources[source] = e(Listing, {
					layoutOrder = string.byte(source),
					transparency = self.props.transparency,

					name = meta.Name,
					description = string.format(
						"%s plugin%s",
						meta.Type,
						if meta.Creator then " by " .. meta.Creator else ""
					),

					onClick = function()
						self.props.onEdit(source, meta, self.props.headlessAPI._permissions[source] or {})
					end,
				})
			end
		end

		return e("Frame", {
			Size = UDim2.new(1, 0, 1, 0),
			BackgroundTransparency = 1,
		}, {
			Navbar = e(Navbar, {
				onBack = self.props.onBack,
				transparency = self.props.transparency,
			}),

			PluginSources = e(ScrollingFrame, {
				size = UDim2.new(1, 0, 1, -47),
				position = UDim2.new(0, 0, 0, 47),
				contentSize = self.contentSize,
				transparency = self.props.transparency,
			}, {
				Layout = e("UIListLayout", {
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,

					[Roact.Change.AbsoluteContentSize] = function(object)
						self.setContentSize(object.AbsoluteContentSize)
					end,
				}),

				Padding = e("UIPadding", {
					PaddingLeft = UDim.new(0, 20),
					PaddingRight = UDim.new(0, 20),
				}),

				Sources = Roact.createFragment(sources),
			}),
		})
	end)
end

return PermissionsPage
