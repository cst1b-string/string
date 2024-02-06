import Link from 'next/link';

export const Logo = () => {
	return (
		<div className="flex items-center gap-x-2">
			<Link href=".">
				<h1 className="text-white text-2xl font-bold">String</h1>
			</Link>
		</div>
	);
}
