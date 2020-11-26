local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local Theme = require(Plugin.App.Theme)

local Header = require(Plugin.App.components.Header)
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
			projectName = "Test Project",
			address = "127.0.0.1:34872",
			transparency = self.props.transparency,
			layoutOrder = 2,
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

return ConnectedPage