import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl;
  
  // For now, disable middleware protection since tokens are in localStorage
  // TODO: Move tokens to httpOnly cookies for better security
  return NextResponse.next();
  
  // Get token from cookies (currently tokens are in localStorage, not cookies)
  const accessToken = request.cookies.get('access_token');
  const isAuthenticated = !!accessToken?.value;
  
  // Public paths that don't require authentication
  const publicPaths = ['/', '/login', '/register', '/forgot-password'];
  const isPublicPath = publicPaths.includes(pathname);
  
  // Protected paths that require authentication
  const protectedPaths = [
    '/dashboard',
    '/exchange',
    '/wallet',
    '/transactions',
    '/profile',
    '/my-rates',
    '/admin',
    '/home'
  ];
  const isProtectedPath = protectedPaths.some(path => pathname.startsWith(path));
  
  // If user is authenticated and tries to access login/register, redirect to home
  if (isAuthenticated && (pathname === '/login' || pathname === '/register')) {
    return NextResponse.redirect(new URL('/home', request.url));
  }
  
  // If user is not authenticated and tries to access protected path, redirect to login
  if (!isAuthenticated && isProtectedPath) {
    const loginUrl = new URL('/login', request.url);
    loginUrl.searchParams.set('from', pathname);
    return NextResponse.redirect(loginUrl);
  }
  
  return NextResponse.next();
}

export const config = {
  matcher: [
    /*
     * Match all request paths except for the ones starting with:
     * - api (API routes)
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     * - public folder
     */
    '/((?!api|_next/static|_next/image|favicon.ico|icons|manifest.json|sw.js|offline.html).*)',
  ],
};