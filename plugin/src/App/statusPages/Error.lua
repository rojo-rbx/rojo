local TextService = game:GetService("TextService")

local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin

local Roact = require(Rojo.Roact)

local e = Roact.createElement

local ErrorPage = Roact.Component:extend("ErrorPage")

function ErrorPage:render()

end

return ErrorPage