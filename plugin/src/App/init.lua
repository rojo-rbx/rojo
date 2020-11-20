local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)
local Log = require(Rojo.Log)

local Assets = require(Plugin.Assets)
local Version = require(Plugin.Version)
local Config = require(Plugin.Config)
local strict = require(Plugin.strict)

local Theme = require(script.Theme)
local Page = require(script.Page)
local StudioToolbar = require(script.components.studio.StudioToolbar)
local StudioToggleButton = require(script.components.studio.StudioToggleButton)
local StudioPluginGui = require(script.components.studio.StudioPluginGui)
local StudioPluginContext = require(script.components.studio.StudioPluginContext)
local statusPages = require(script.statusPages)

local AppStatus = strict("AppStatus", {
	NotConnected = "NotConnected",
	Settings = "Settings",
	Connecting = "Connecting",
	Connected = "Connected",
	Error = "Error",
})

local e = Roact.createElement

local App = Roact.Component:extend("App")

function App:init()
	self:setState({
		appStatus = AppStatus.NotConnected,
		guiEnabled = false,
	})

	spawn(function()
		while true do
			wait(1)
			self:setState({ appStatus = AppStatus.Connected })
			wait(1)
			self:setState({ appStatus = AppStatus.NotConnected })
		end
	end)
end

function App:render()
	local pages = {}

	for _, appStatus in pairs(AppStatus) do
		pages[appStatus] = e(Page, {
			component = statusPages[appStatus],
			active = self.state.appStatus == appStatus,
		})
	end

	local name = "Rojo " .. Version.display(Config.version)
	return e(StudioPluginContext.Provider, {
		value = self.props.plugin,
	}, {
		e(Theme.StudioProvider, nil, {
			gui = e(StudioPluginGui, {
				id = name,
				title = name,
				active = self.state.guiEnabled,

				initDockState = Enum.InitialDockState.Right,
				initEnabled = false,
				overridePreviousState = false,
				floatingSize = Vector2.new(360, 190),
				minimumSize = Vector2.new(360, 190),

				onInitialState = function(initialState)
					self:setState({
						guiEnabled = initialState,
					})
				end,
			}, pages),

			toolbar = e(StudioToolbar, {
				name = name,
			}, {
				button = e(StudioToggleButton, {
					name = "Rojo",
					tooltip = "Show or hide the Rojo panel",
					icon = Assets.Images.Icon,
					enabled = true,
					onClick = function()
						self:setState(function(state)
							return {
								guiEnabled = not state.guiEnabled,
							}
						end)
					end,
				})
			}),
		})
	})
end

return App