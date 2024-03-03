import { Logo } from "./logo";
import { SettingsButton } from "./settings-button";
import { SignInButton } from "./sign-in-button";

function LoginOrSettingsButton({ loggedIn }: { loggedIn: boolean }) {
	if (loggedIn) {
		return <SettingsButton />;
	}
	return <SignInButton />;
} 

//TODO: Undo auto login

export const Navbar = () => (
	<div className="w-full h-20 bg-[#191970] sticky top-0">
		<div className="container mx-auto px-4 h-full">
			<div className="flex justify-between items-center h-full">
				<Logo />
				<LoginOrSettingsButton loggedIn={true} />
			</div>
		</div>
	</div>
);
