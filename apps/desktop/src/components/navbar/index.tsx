import { Logo } from "./logo";
import { SettingsButton } from "./settings-button";

export const Navbar = () => {
	return (
		<div className="w-full h-[60px] bg-darkNavbar sticky top-0 shadow-md">
			<div className="container mx-auto px-4 h-full">
				<div className="flex justify-between items-center h-full">
					<Logo />
					<SettingsButton />
				</div>
			</div>
		</div>
	);
};
