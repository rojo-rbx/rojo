local Roact = require(script:FindFirstAncestor("Rojo").Roact)

local ConnectPanel = require(script.Parent.ConnectPanel)

local e = Roact.createElement

local App = Roact.Component:extend("App")

function App:render()
	return e("ScreenGui", nil, {
		ConnectPanel = e(ConnectPanel),
	})
end

return App