local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local Timer = require(Plugin.Timer)
local PatchTree = require(Plugin.PatchTree)
local Settings = require(Plugin.Settings)
local Theme = require(Plugin.App.Theme)
local TextButton = require(Plugin.App.Components.TextButton)
local Header = require(Plugin.App.Components.Header)
local StudioPluginGui = require(Plugin.App.Components.Studio.StudioPluginGui)
local Tooltip = require(Plugin.App.Components.Tooltip)
local PatchVisualizer = require(Plugin.App.Components.PatchVisualizer)
local StringDiffVisualizer = require(Plugin.App.Components.StringDiffVisualizer)
local TableDiffVisualizer = require(Plugin.App.Components.TableDiffVisualizer)

local e = Roact.createElement

local ConfirmingPage = Roact.Component:extend("ConfirmingPage")

function ConfirmingPage:init()
	self.contentSize, self.setContentSize = Roact.createBinding(0)
	self.containerSize, self.setContainerSize = Roact.createBinding(Vector2.new(0, 0))

	self:setState({
		patchTree = nil,
		showingStringDiff = false,
		oldString = "",
		newString = "",
		showingTableDiff = false,
		oldTable = {},
		newTable = {},
	})

	if self.props.confirmData and self.props.confirmData.patch and self.props.confirmData.instanceMap then
		self:buildPatchTree()
	end
end

function ConfirmingPage:didUpdate(prevProps)
	if prevProps.confirmData ~= self.props.confirmData then
		self:buildPatchTree()
	end
end

function ConfirmingPage:buildPatchTree()
	Timer.start("ConfirmingPage:buildPatchTree")
	self:setState({
		patchTree = PatchTree.build(
			self.props.confirmData.patch,
			self.props.confirmData.instanceMap,
			{ "Property", "Current", "Incoming" }
		),
	})
	Timer.stop()
end

function ConfirmingPage:render()
	return Theme.with(function(theme)
		local pageContent = Roact.createFragment({
			Header = e(Header, {
				transparency = self.props.transparency,
				layoutOrder = 1,
			}),

			Title = e("TextLabel", {
				Text = string.format(
					"Sync changes for project '%s':",
					self.props.confirmData.serverInfo.projectName or "UNKNOWN"
				),
				LayoutOrder = 2,
				Font = Enum.Font.Gotham,
				LineHeight = 1.2,
				TextSize = 14,
				TextColor3 = theme.TextColor,
				TextXAlignment = Enum.TextXAlignment.Left,
				TextTransparency = self.props.transparency,
				Size = UDim2.new(1, 0, 0, 20),
				BackgroundTransparency = 1,
			}),

			PatchVisualizer = e(PatchVisualizer, {
				size = UDim2.new(1, 0, 1, -150),
				transparency = self.props.transparency,
				layoutOrder = 3,

				patchTree = self.state.patchTree,

				showStringDiff = function(oldString: string, newString: string)
					self:setState({
						showingStringDiff = true,
						oldString = oldString,
						newString = newString,
					})
				end,
				showTableDiff = function(oldTable: { [any]: any? }, newTable: { [any]: any? })
					self:setState({
						showingTableDiff = true,
						oldTable = oldTable,
						newTable = newTable,
					})
				end,
			}),

			Buttons = e("Frame", {
				Size = UDim2.new(1, 0, 0, 34),
				LayoutOrder = 4,
				BackgroundTransparency = 1,
			}, {
				Abort = e(TextButton, {
					text = "Abort",
					style = "Bordered",
					transparency = self.props.transparency,
					layoutOrder = 1,
					onClick = self.props.onAbort,
				}, {
					Tip = e(Tooltip.Trigger, {
						text = "Stop the connection process",
					}),
				}),

				Reject = if Settings:get("twoWaySync")
					then e(TextButton, {
						text = "Reject",
						style = "Bordered",
						transparency = self.props.transparency,
						layoutOrder = 2,
						onClick = self.props.onReject,
					}, {
						Tip = e(Tooltip.Trigger, {
							text = "Push Studio changes to the Rojo server",
						}),
					})
					else nil,

				Accept = e(TextButton, {
					text = "Accept",
					style = "Solid",
					transparency = self.props.transparency,
					layoutOrder = 3,
					onClick = self.props.onAccept,
				}, {
					Tip = e(Tooltip.Trigger, {
						text = "Pull Rojo server changes to Studio",
					}),
				}),

				Layout = e("UIListLayout", {
					HorizontalAlignment = Enum.HorizontalAlignment.Right,
					FillDirection = Enum.FillDirection.Horizontal,
					SortOrder = Enum.SortOrder.LayoutOrder,
					Padding = UDim.new(0, 10),
				}),
			}),

			Layout = e("UIListLayout", {
				HorizontalAlignment = Enum.HorizontalAlignment.Center,
				VerticalAlignment = Enum.VerticalAlignment.Center,
				FillDirection = Enum.FillDirection.Vertical,
				SortOrder = Enum.SortOrder.LayoutOrder,
				Padding = UDim.new(0, 10),
			}),

			Padding = e("UIPadding", {
				PaddingLeft = UDim.new(0, 20),
				PaddingRight = UDim.new(0, 20),
			}),

			StringDiff = e(StudioPluginGui, {
				id = "Rojo_ConfirmingStringDiff",
				title = "String diff",
				active = self.state.showingStringDiff,
				isEphemeral = true,

				initDockState = Enum.InitialDockState.Float,
				overridePreviousState = true,
				floatingSize = Vector2.new(500, 350),
				minimumSize = Vector2.new(400, 250),

				zIndexBehavior = Enum.ZIndexBehavior.Sibling,

				onClose = function()
					self:setState({
						showingStringDiff = false,
					})
				end,
			}, {
				TooltipsProvider = e(Tooltip.Provider, nil, {
					Tooltips = e(Tooltip.Container, nil),
					Content = e("Frame", {
						Size = UDim2.fromScale(1, 1),
						BackgroundTransparency = 1,
					}, {
						e(StringDiffVisualizer, {
							size = UDim2.new(1, -10, 1, -10),
							position = UDim2.new(0, 5, 0, 5),
							anchorPoint = Vector2.new(0, 0),
							transparency = self.props.transparency,

							oldString = self.state.oldString,
							newString = self.state.newString,
						}),
					}),
				}),
			}),

			TableDiff = e(StudioPluginGui, {
				id = "Rojo_ConfirmingTableDiff",
				title = "Table diff",
				active = self.state.showingTableDiff,
				isEphemeral = true,

				initDockState = Enum.InitialDockState.Float,
				overridePreviousState = true,
				floatingSize = Vector2.new(500, 350),
				minimumSize = Vector2.new(400, 250),

				zIndexBehavior = Enum.ZIndexBehavior.Sibling,

				onClose = function()
					self:setState({
						showingTableDiff = false,
					})
				end,
			}, {
				TooltipsProvider = e(Tooltip.Provider, nil, {
					Tooltips = e(Tooltip.Container, nil),
					Content = e("Frame", {
						Size = UDim2.fromScale(1, 1),
						BackgroundTransparency = 1,
					}, {
						e(TableDiffVisualizer, {
							size = UDim2.new(1, -10, 1, -10),
							position = UDim2.new(0, 5, 0, 5),
							anchorPoint = Vector2.new(0, 0),
							transparency = self.props.transparency,

							oldTable = self.state.oldTable,
							newTable = self.state.newTable,
						}),
					}),
				}),
			}),
		})

		if self.props.createPopup then
			return e(StudioPluginGui, {
				id = "Rojo_DiffSync",
				title = string.format(
					"Confirm sync for project '%s':",
					self.props.confirmData.serverInfo.projectName or "UNKNOWN"
				),
				active = true,
				isEphemeral = true,

				initDockState = Enum.InitialDockState.Float,
				overridePreviousState = false,
				floatingSize = Vector2.new(500, 350),
				minimumSize = Vector2.new(400, 250),

				zIndexBehavior = Enum.ZIndexBehavior.Sibling,

				onClose = self.props.onAbort,
			}, {
				Tooltips = e(Tooltip.Container, nil),
				Content = e("Frame", {
					Size = UDim2.fromScale(1, 1),
					BackgroundTransparency = 1,
				}, pageContent),
			})
		end

		return pageContent
	end)
end

return ConfirmingPage
