local Rojo = script:FindFirstAncestor("Rojo")
local Plugin = Rojo.Plugin
local Packages = Rojo.Packages

local Roact = require(Packages.Roact)

local BorderedContainer = require(Plugin.App.Components.BorderedContainer)
local Array = require(script:FindFirstChild("Array"))
local Dictionary = require(script:FindFirstChild("Dictionary"))

local e = Roact.createElement

local TableDiffVisualizer = Roact.Component:extend("TableDiffVisualizer")

function TableDiffVisualizer:render()
	local oldTable, newTable = self.props.oldTable or {}, self.props.newTable or {}

	-- Ensure we're diffing tables, not mixing types
	if type(oldTable) ~= "table" then
		oldTable = {}
	end
	if type(newTable) ~= "table" then
		newTable = {}
	end

	local isArray = next(newTable) == 1 or next(oldTable) == 1

	return e(BorderedContainer, {
		size = self.props.size,
		position = self.props.position,
		anchorPoint = self.props.anchorPoint,
		transparency = self.props.transparency,
	}, {
		Content = if isArray
			then e(Array, {
				oldTable = oldTable,
				newTable = newTable,
				transparency = self.props.transparency,
			})
			else e(Dictionary, {
				oldTable = oldTable,
				newTable = newTable,
				transparency = self.props.transparency,
			}),
	})
end

return TableDiffVisualizer
