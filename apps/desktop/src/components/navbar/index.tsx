import { Logo } from './logo';
import { LoginButton } from './login-button';
import { SettingsButton } from './settings-button';

function LoginOrSettingsButton( { loggedIn } : { loggedIn: boolean }) {
  if (loggedIn) {
	return <SettingsButton />;
  } else {
	return <LoginButton />;
  }
}

export const Navbar = () => {
	return (
		<div className="w-full h-20 bg-[#191970] sticky top-0">
        <div className="container mx-auto px-4 h-full">
          <div className="flex justify-between items-center h-full">
            <Logo />
			{LoginOrSettingsButton({ loggedIn: true })}
          </div>
        </div>
      </div>
	);
}
