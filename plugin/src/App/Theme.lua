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

local baseTheme = strict("BaseTheme", {
	Brand = hexColor(0xE13835),
})

local lightTheme = strict("LightTheme", {

})

local darkTheme = strict("DarkTheme", {

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