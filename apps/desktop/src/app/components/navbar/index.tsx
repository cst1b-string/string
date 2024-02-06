import Link from 'next/link';
import { Logo } from './logo';
import { LoginButton } from './login-button';

export const Navbar = () => {
	return (
		<div className="w-full h-20 bg-[#191970] sticky top-0">
        <div className="container mx-auto px-4 h-full">
          <div className="flex justify-between items-center h-full">
            <LoginButton />
            <Logo />
			<Link href='/settings' className='text-white'>Settings</Link>
          </div>
        </div>
      </div>
	);
}
