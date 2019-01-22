--[[
	joinBindings is a crazy hack that allows combining multiple Roact bindings
	in the same spirit as `map`.

	It's implemented in terms of Roact internals that will probably break at
	some point; please don't do that or use this module in your own code!
]]

local Binding = require(script:FindFirstAncestor("Rojo").Roact.Binding)

local function evaluate(fun, bindings)
	local input = {}

	for index, binding in ipairs(bindings) do
		input[index] = binding:getValue()
	end

	return fun(unpack(input, 1, #bindings))
end

local function joinBindings(bindings, joinFunction)
	local initialValue = evaluate(joinFunction, bindings)
	local binding, setValue = Binding.create(initialValue)

	for _, binding in ipairs(bindings) do
		Binding.subscribe(binding, function()
			setValue(evaluate(joinFunction, bindings))
		end)
	end

	return binding
end

return joinBindings