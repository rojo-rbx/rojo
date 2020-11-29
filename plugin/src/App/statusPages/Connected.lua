local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Theme = require(Plugin.App.Theme)
local Assets = require(Plugin.Assets)

local Header = require(Plugin.App.components.Header)
local IconButton = require(Plugin.App.components.IconButton)
local BorderedContainer = require(Plugin.App.components.BorderedContainer)

local e = Roact.createElement

local function ConnectionDetails(props)
	return Theme.with(function(theme)
		return e(BorderedContainer, {
			transparency = props.transparency,
			size = UDim2.new(1, 0, 0, 70),
			layoutOrder = props.layoutOrder,
		}, {
			TextContainer = e("Frame", {
				Size = UDim2.new(1, 0, 1, 0),
				BackgroundTransparency = 1
			}, {
				ProjectName = e("TextLabel", {
					Text = props.projectName,
					Font = Enum.Font.GothamBold,
					TextSize = 20,
					TextColor3 = theme.ConnectionDetails.ProjectNameColor,
					TextTransparency = props.transparency,
					TextXAlignment = Enum.TextXAlignment.Left,

					Size = UDim2.new(1, 0, 0, 20),

					LayoutOrder = 1,
					BackgroundTransparency = 1,
				}),

				Address = e("TextLabel", {
					Text = props.address,
					Font = Enum.Font.Code,
					TextSize = 15,
					TextColor3 = theme.ConnectionDetails.AddressColor,
					TextTransparency = props.transparency,
					TextXAlignment = Enum.TextXAlignment.Left,

					Size = UDim2.new(1, 0, 0, 15),

					LayoutOrder = 2,
					BackgroundTransparency = 1,
				}),

				Layout = e("UIListLayout", {
					VerticalAlignment = Enum.VerticalAlignment.Center,
					FillDirection = Enum.FillDirection.Vertical,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 6),
				}),
			}),

			Disconnect = e(IconButton, {
				icon = Assets.Images.Icons.Close,
				iconSize = 24,
				color = theme.ConnectionDetails.DisconnectColor,
				transparency = props.transparency,

				position = UDim2.new(1, 0, 0.5, 0),
				anchorPoint = Vector2.new(1, 0.5),

				onClick = props.onDisconnect,
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 17),
				PaddingRight = UDim.new(0, 15),
			}),
		})
	end)
end

local ConnectedPage = Roact.Component:extend("ConnectedPage")

function ConnectedPage:render()
	return Roact.createFragment({
		Header = e(Header, {
			transparency = self.props.transparency,
			layoutOrder = 1,
		}),

		ConnectionDetails = e(ConnectionDetails, {
			projectName = self.state.projectName,
			address = self.state.address,
			transparency = self.props.transparency,
			layoutOrder = 2,

			onDisconnect = self.props.onDisconnect,
		}),

		Layout = e("UIListLayout", {
			VerticalAlignment = Enum.VerticalAlignment.Center,
			FillDirection = Enum.FillDirection.Vertical,
			SortOrder = Enum.SortOrder.LayoutOrder,
			Padding = UDim.new(0, 10),
		}),

		Padding = e("UIPadding", {
			PaddingLeft = UDim.new(0, 20),
			PaddingRight = UDim.new(0, 20),
		}),
	})
end

function ConnectedPage.getDerivedStateFromProps(props)
	return {
		projectName = props.projectName,
		address = props.address,
	}
end

return ConnectedPage