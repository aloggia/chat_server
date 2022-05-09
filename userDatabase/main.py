import sqlite3
import string
import hashlib
import os
import random
import sys


# Create/open the credentials database
def create_db():
    try:
        conn = sqlite3.connect("credentials.db")
        c = conn.cursor()
        c.execute(''' CREATE TABLE credentials 
        (
        username TEXT PRIMARY KEY,
        password TEXT  
        )
        ''')
        conn.commit()
        return True
    except BaseException:
        return False
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()


# Add a user to credentials.db, both username and password have already been sanatized
def add_user(username, passwd):
    data_to_insert = [(username, hash_passwd(passwd))]
    try:
        conn = sqlite3.connect('credentials.db')
        c = conn.cursor()
        c.executemany("INSERT INTO credentials VALUES (? ,?)", data_to_insert)
        conn.commit()
    except sqlite3.IntegrityError:
        # We don't want to print any error messages, because the outer server is only looking for a 1 or 0
        # print("Error. Tried to add duplicate record")
        pass
    else:
        # print("Success")
        pass
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()


# Take in a raw username, and check if that user is registered in the database
def get_username(username):
    try:
        user = sanitize(username)
        conn = sqlite3.connect('credentials.db')
        c = conn.cursor()
        query = """SELECT * FROM credentials where username = ?"""
        c.execute(query, (user,))
        records = c.fetchone()
        names_to_return = records[0]
    except sqlite3.DatabaseError:
        # print("Error. Could not retrieve data")
        pass
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()
        return names_to_return


# This is a testing function to get a password from a username, it is never run unless the programmer is debugging
def get_passwd(username):
    try:
        user = sanitize(username)
        conn = sqlite3.connect('credentials.db')
        c = conn.cursor()
        query = """SELECT * FROM credentials where username = ?"""
        c.execute(query, (user,))
        records = c.fetchone()
        passwd = records[1]
    except sqlite3.DatabaseError:
        # print("Error. Could not retrieve data")
        pass
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()
        return passwd


# Check if a user is in credentials.db at all
def is_user_in_system(username):
    try:
        user = sanitize(username)
        is_in = False
        conn = sqlite3.connect('credentials.db')
        c = conn.cursor()
        # Get all rows in credentials, and check each one for the given username
        for row in c.execute("SELECT * FROM credentials"):
            if user == row[0]:
                is_in = True
    except sqlite3.DatabaseError:
        # print("Error. Could not retrieve data")
        pass
    finally:
        if c is not None:
            c.close()
        if conn is not None:
            conn.close()
        return is_in


# Take in the hashed_pw from credentials.db, and the plaintext password
# add the stored password salt to the plaintext, hash it with sha256, and compare that hashed value with the stored
# hash. If they are equal then the plaintext password is correct
def authenticate(hashed_pw, plaintext):
    salt = hashed_pw[:40]
    saved_hash = hashed_pw[40:]
    hashable = salt + plaintext.encode('utf-8')
    this_hash = hashlib.sha256(hashable).hexdigest().encode('utf-8')
    return this_hash == saved_hash


# Generate 40 random chars for the password salt
# Salting a hash adds security
# Add the randomly generated salt to the plaintext password, hash it with sha256, then concatinate the salt to the front
# of the hashed value
def hash_passwd(plaintext):
    salt = os.urandom(40)
    hashable = salt + plaintext.encode('utf-8')
    hashed = hashlib.sha256(hashable).hexdigest().encode('utf-8')
    return salt + hashed


# To prevent sql injections we can remove all special chars that are used in sql queries
def sanitize(in_word):
    bad_chars = ['%', '*', ';', "'", '_', '^', '-', '[', ']', '"']
    list_pos = 0
    # split the word to sanitize into an array of letters
    search_array = [char for char in in_word]
    # Compare each letter with all the bad chars. If a bad char is found, delete it
    for illegal_char in bad_chars:
        list_pos = 0
        for search_char in search_array:
            if illegal_char == search_char:
                search_array[list_pos] = ''
            list_pos += 1
    # Join the sanitized array back into a string, and return the string
    new_search_term = ''.join(search_array)
    return new_search_term


if __name__ == '__main__':
    # Command line args:
    # Register user
    # check user
    # log in
    create_db()
    command_arg = sys.argv[1]
    username = sys.argv[2]
    password = sys.argv[3]

    if command_arg == 'register':
        # Run register user
        add_user(sanitize(username), password)
    if command_arg == 'check_user':
        if is_user_in_system(username):
            print('1')
        else:
            print('0')
    if command_arg == 'log_in':
        # Check if user is in system
        if is_user_in_system(username):
            # User is in system, Check the password
            stored_password = get_passwd(username)
            if authenticate(stored_password, password):
                # Password is correct, pass the value 1 back to server program, the server interprets 1 as true
                print("1")
            else:
                # Password is wrong
                print('0')
        else:
            print('0')
