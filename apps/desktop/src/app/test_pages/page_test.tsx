//import Image from "next/image";

export default function Home() {
	return (
		<main className="flex min-h-screen flex-col items-center justify-between p-24">
			<div className="font-mono text-sm">
				<h2 className={`text-3xl`}>
					<code className="font-bold"> String </code>
				</h2>
			</div>
			<div className="font-mono text-sm">
				<p className="fixed left-0 top-0 flex w-full justify-center border-b border-gray-300 bg-gradient-to-b from-zinc-200 pb-6 pt-8 backdrop-blur-2xl dark:border-neutral-800 dark:bg-zinc-800/30 dark:from-inherit lg:static lg:w-auto  lg:rounded-xl lg:border lg:bg-gray-200 lg:p-4 lg:dark:bg-zinc-800/30">
					Welcome to&nbsp;
					<code className="font-mono font-bold">String, </code>
					&nbsp;a decentralised social network.
				</p>

				<div className="font-mono text-sm">
					<p className="justify-center border-b border-gray-300 bg-gradient-to-b from zinc-200">
						{" "}
						Test pn
					</p>
				</div>
			</div>
		</main>
	);
}
