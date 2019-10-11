print("test-plugin initializing...")

return function(nextDispatch, entry)
	if entry:isDirectory() then
		return nextDispatch(entry)
	end

	local name = entry:fileName()
	local instanceName = name:match("(.-)%.moon$")

	if instanceName == nil then
		return nextDispatch(entry)
	end

	return rojo.instance({
		Name = instanceName,
		ClassName = "ModuleScript",
		Source = compileMoonScript(entry:contents()),
	})
end