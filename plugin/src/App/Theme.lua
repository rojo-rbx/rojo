--[[
	Theming system taking advantage of Roact's new context API.
	Doesn't use colors provided by Studio and instead just branches on theme
	name. This isn't exactly best practice.
]]

-- Studio does not exist outside Roblox Studio, so we'll lazily initialize it
-- when possible.
local _Studio
local function getStudio()
	if _Studio == nil then
		_Studio = settings():GetService("Studio")
	end

	return _Studio
end

local Rojo = script:FindFirstAncestor("Rojo")

local Roact = require(Rojo.Roact)
local Log = require(Rojo.Log)

local strict = require(script.Parent.Parent.strict)

-- Copying hex colors back and forth from design programs is faster
local function hexColor(decimal)
	local red = bit32.band(bit32.rshift(decimal, 16), 2^8 - 1)
	local green = bit32.band(bit32.rshift(decimal, 8), 2^8 - 1)
	local blue = bit32.band(decimal, 2^8 - 1)

	return Color3.fromRGB(red, green, blue)
end

local BRAND_COLOR = hexColor(0xE13835)

local lightTheme = strict("LightTheme", {
	Background = hexColor(0xF0F0F0),

	Button = {
		Solid = {
			ActionFill = hexColor(0xFFFFFF),
			ActionFillTransparency = 0.8,

			Enabled = {
				Text = hexColor(0xFFFFFF),
				Background = BRAND_COLOR,
			},

			Disabled = {
				Text = hexColor(0xFFFFFF),
				Background = BRAND_COLOR,
			},
		},

		Bordered = {
			ActionFill = hexColor(0x000000),
			ActionFillTransparency = 0.9,

			Enabled = {
				Text = hexColor(0x393939),
				Border = hexColor(0xACACAC),
			},

			Disabled = {
				Text = hexColor(0x393939),
				Border = hexColor(0xACACAC),
			},
		},
	},

	AddressEntry = {
		Text = hexColor(0x000000),
		Placeholder = hexColor(0x8C8C8C)
	},

	BorderedContainer = {
		Border = hexColor(0xCBCBCB),
		Background = hexColor(0xE0E0E0),
	},

	Throbber = {
		Foreground = BRAND_COLOR,
		Background = hexColor(0xE0E0E0),
	},

	ConnectionDetails = {
		ProjectName = hexColor(0x00000),
		Address = hexColor(0x00000),
	},

	Header = {
		Logo = BRAND_COLOR,
		Version = hexColor(0x727272)
	},
})

local darkTheme = strict("DarkTheme", {
	Background = hexColor(0x272727),

	Button = {
		Solid = {
			ActionFill = hexColor(0xFFFFFF),
			ActionFillTransparency = 0.8,

			Enabled = {
				Text = hexColor(0xFFFFFF),
				Background = BRAND_COLOR,
			},

			Disabled = {
				Text = hexColor(0xFFFFFF),
				Background = BRAND_COLOR,
			},
		},

		Bordered = {
			ActionFill = hexColor(0xFFFFFF),
			ActionFillTransparency = 0.9,

			Enabled = {
				Text = hexColor(0xDBDBDB),
				Border = hexColor(0x535353),
			},

			Disabled = {
				Text = hexColor(0xDBDBDB),
				Border = hexColor(0x535353),
			},
		},
	},

	AddressEntry = {
		Text = hexColor(0xFFFFFF),
		Placeholder = hexColor(0x717171)
	},

	BorderedContainer = {
		Border = hexColor(0x535353),
		Background = hexColor(0x323232),
	},

	Throbber = {
		Foreground = BRAND_COLOR,
		Background = hexColor(0x323232),
	},

	ConnectionDetails = {
		ProjectName = hexColor(0xFFFFFF),
		Address = hexColor(0xFFFFFF),
	},

	Header = {
		Logo = hexColor(0xFFFFFF),
		Version = hexColor(0xD3D3D3)
	},
})

local Context = Roact.createContext(lightTheme)

local StudioProvider = Roact.Component:extend("StudioProvider")

-- Pull the current theme from Roblox Studio and update state with it.
function StudioProvider:updateTheme()
	local studioTheme = getStudio().Theme

	if studioTheme.Name == "Light" then
		self:setState({
			theme = lightTheme,
		})
	elseif studioTheme.Name == "Dark" then
		self:setState({
			theme = darkTheme,
		})
	else
		Log.warn("Unexpected theme '{}'' -- falling back to light theme!", studioTheme.Name)

		self:setState({
			theme = lightTheme,
		})
	end
end

function StudioProvider:init()
	self:updateTheme()
end

function StudioProvider:render()
	return Roact.createElement(Context.Provider, {
		value = self.state.theme,
	}, self.props[Roact.Children])
end

function StudioProvider:didMount()
	self.connection = getStudio().ThemeChanged:Connect(function()
		self:updateTheme()
	end)
end

function StudioProvider:willUnmount()
	self.connection:Disconnect()
end

local function with(callback)
	return Roact.createElement(Context.Consumer, {
		render = callback,
	})
end

return {
	StudioProvider = StudioProvider,
	Consumer = Context.Consumer,
	with = with,
}