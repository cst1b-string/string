import Link from 'next/link';

export const SignInButton = () => {
	return (
		<button className="bg-white text-[#191970] px-4 py-2 rounded-md">
			<Link href="/signIn">Sign In</Link>
		</button>
	);
}
