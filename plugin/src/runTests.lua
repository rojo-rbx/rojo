return function(TestEZ)
	local Rojo = script.Parent.Parent
	local Packages = Rojo.Packages

	return TestEZ.TestBootstrap:run({ Rojo.Plugin, Packages.Http, Packages.Log, Packages.RbxDom })
end
