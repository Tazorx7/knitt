import logo from "../assets/logo.png";

export default function Header() {
  return (
    <header className="flex items-center gap-2 px-4 py-2 border-b">
      <img src={logo} alt="knitt" className="w-6 h-6" />
      <span className="font-semibold">knitt</span>
    </header>
  );
}
